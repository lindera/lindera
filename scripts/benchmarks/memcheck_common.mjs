// Shared measurement logic for the JS bindings' memory-regression checks.
//
// The problem being guarded against (see #922, #930): when a binding returns
// class instances, their native memory is released by a finalizer that the
// host defers to the event loop. A synchronous loop that never yields
// therefore accumulates native memory even under forced GC, while the JS heap
// stays flat. Plain objects are owned entirely by the JS heap, so the same
// loop stays flat.
//
// Assertion strategy: rather than a fixed MB ceiling — which is fragile
// across CI runners with different allocators and dictionary sizes — compare
// how much RSS grows over the first half of the run against the second half.
// Deferred-finalizer accumulation keeps growing roughly linearly, so the
// second half grows about as much as the first. Once the leak is gone, the
// second half flattens out near zero. A generous absolute ceiling is kept as
// a safety net for pathological cases.

/// Runs `tokenizeOnce` repeatedly and reports how RSS evolves.
///
/// Requires `--expose-gc`: `global.gc()` runs between iterations so the
/// measurement reflects memory that could not be reclaimed, matching #922's
/// methodology.
///
/// @param {object} options
/// @param {() => void} options.tokenizeOnce - One full tokenize call.
/// @param {number} options.iterations - Measured iterations (excludes warmup).
/// @param {number} options.warmup - Unmeasured iterations run first, so
///   one-time allocator growth is not counted as a leak.
/// @returns {{ first: number, second: number, total: number, samples: number[] }}
///   Growth in bytes over the first and second halves, the total, and the raw
///   per-iteration RSS samples.
export function measureRssGrowth({ tokenizeOnce, iterations, warmup }) {
  if (typeof global.gc !== "function") {
    throw new Error(
      "memcheck requires --expose-gc (run node with --expose-gc)",
    );
  }

  for (let i = 0; i < warmup; i++) {
    tokenizeOnce();
  }
  global.gc();

  const samples = [process.memoryUsage().rss];
  for (let i = 0; i < iterations; i++) {
    tokenizeOnce();
    global.gc();
    samples.push(process.memoryUsage().rss);
  }

  const mid = Math.floor(iterations / 2);
  const first = samples[mid] - samples[0];
  const second = samples[iterations] - samples[mid];
  return {
    first,
    second,
    total: samples[iterations] - samples[0],
    samples,
  };
}

/// Formats a byte count as MB for human-readable output.
///
/// @param {number} bytes
/// @returns {string}
export function mb(bytes) {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/// Applies the regression criteria and reports the verdict on stdout.
///
/// @param {object} options
/// @param {string} options.label - Binding name, for the output lines.
/// @param {ReturnType<typeof measureRssGrowth>} options.growth
/// @param {number} options.tolerance - Allowed ratio of second-half growth to
///   first-half growth. Accumulation holds this near 1.0 or above; a fixed
///   binding drops it to roughly 0.
/// @param {number} options.ceilingBytes - Absolute total-growth safety net.
/// @returns {boolean} Whether the check passed.
export function reportVerdict({ label, growth, tolerance, ceilingBytes }) {
  const { first, second, total } = growth;
  console.log(`${label}\tfirst-half\t${mb(first)}`);
  console.log(`${label}\tsecond-half\t${mb(second)}`);
  console.log(`${label}\ttotal\t${mb(total)}`);

  // A first half that barely moved means there is no trend to compare
  // against; fall back to the absolute ceiling alone. Threshold is well above
  // ordinary allocator jitter but far below real accumulation.
  const TREND_FLOOR_BYTES = 16 * 1024 * 1024;
  const failures = [];

  if (first > TREND_FLOOR_BYTES) {
    const ratio = second / first;
    console.log(`${label}\tratio\t${ratio.toFixed(3)}`);
    if (ratio > tolerance) {
      failures.push(
        `RSS keeps growing: second half grew ${mb(second)} against ` +
          `${mb(first)} in the first (ratio ${ratio.toFixed(3)} > ${tolerance}). ` +
          `This is the deferred-finalizer accumulation pattern from #922.`,
      );
    }
  } else {
    console.log(`${label}\tratio\tn/a (first half below trend floor)`);
  }

  if (total > ceilingBytes) {
    failures.push(
      `total RSS growth ${mb(total)} exceeds the ceiling ${mb(ceilingBytes)}`,
    );
  }

  for (const failure of failures) {
    console.error(`FAIL: ${failure}`);
  }
  if (failures.length === 0) {
    console.log(`${label}\tOK`);
  }
  return failures.length === 0;
}
