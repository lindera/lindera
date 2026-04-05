<?php declare(strict_types=1);
/*
 * This file is part of sebastian/git-state.
 *
 * (c) Sebastian Bergmann <sebastian@phpunit.de>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */
namespace SebastianBergmann\GitState;

use function assert;
use function explode;
use function preg_split;
use function str_contains;
use function str_replace;
use function str_starts_with;
use function trim;

/**
 * @no-named-arguments Parameter names are not covered by the backward compatibility promise for sebastian/git-state
 */
final readonly class Builder
{
    private GitCommandRunner $runner;

    public function __construct(?GitCommandRunner $runner = null)
    {
        if ($runner === null) {
            $runner = new GitCommandRunner(
                new ShellCommandRunnerImplementation,
            );
        }

        $this->runner = $runner;
    }

    public function build(): false|State
    {
        $buffer = $this->runner->run('remote show -n');

        if ($buffer === false) {
            return false;
        }

        if (!str_contains($buffer, 'origin')) {
            return false;
        }

        $buffer = $this->runner->run('remote show -n origin');

        if ($buffer === false) {
            return false;
        }

        $lines = preg_split("/\r\n|\n|\r/", $buffer);

        if (!isset($lines[1]) || !str_starts_with($lines[1], '  Fetch URL: ')) {
            return false;
        }

        $originUrl = trim(str_replace('  Fetch URL: ', '', $lines[1]));

        if (str_contains($originUrl, '@')) {
            $tmp = explode('@', $originUrl);

            assert(isset($tmp[1]));

            $originUrl = $tmp[1];
        }

        assert($originUrl !== '');

        $branch = $this->runner->run('rev-parse --abbrev-ref HEAD');

        if ($branch === false) {
            return false;
        }

        assert($branch !== '');

        $commit = $this->runner->run('rev-parse HEAD');

        if ($commit === false) {
            return false;
        }

        assert($commit !== '');

        $status = $this->runner->run('status --porcelain');

        if ($status === false) {
            return false;
        }

        return new State($originUrl, $branch, $commit, $status === '', $status);
    }
}
