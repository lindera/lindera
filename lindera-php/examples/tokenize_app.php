<?php

/**
 * Web-based morphological analysis application.
 *
 * A simple PHP web app that provides a browser UI for Lindera tokenization,
 * similar to lindera-wasm's web example.
 *
 * Usage:
 *   cargo build -p lindera-php --features embed-ipadic
 *   php -d extension=target/debug/liblindera_php.so -S localhost:8080 lindera-php/examples/tokenize_app.php
 *
 * Then open http://localhost:8080 in your browser.
 */

// Handle JSON API request
if ($_SERVER['REQUEST_METHOD'] === 'POST' && isset($_SERVER['CONTENT_TYPE']) && str_contains($_SERVER['CONTENT_TYPE'], 'application/json')) {
    header('Content-Type: application/json; charset=utf-8');

    $input = json_decode(file_get_contents('php://input'), true);
    $text = $input['text'] ?? '';
    $mode = $input['mode'] ?? 'normal';
    $dictionary = $input['dictionary'] ?? 'embedded://ipadic';

    if (empty($text)) {
        echo json_encode(['error' => 'Text is required']);
        exit;
    }

    try {
        $dict = Lindera\Dictionary::load($dictionary);
        $tokenizer = new Lindera\Tokenizer($dict, $mode);
        $tokens = $tokenizer->tokenize($text);

        $results = [];
        foreach ($tokens as $token) {
            $results[] = [
                'surface' => $token->surface,
                'byte_start' => $token->byte_start,
                'byte_end' => $token->byte_end,
                'position' => $token->position,
                'word_id' => $token->word_id,
                'is_unknown' => $token->is_unknown,
                'details' => $token->details,
            ];
        }

        echo json_encode([
            'tokens' => $results,
            'version' => Lindera\Dictionary::version(),
        ], JSON_UNESCAPED_UNICODE);
    } catch (\Throwable $e) {
        http_response_code(500);
        echo json_encode(['error' => $e->getMessage()]);
    }
    exit;
}

// Serve HTML page
$version = Lindera\Dictionary::version();
?>
<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Lindera PHP v<?= htmlspecialchars($version) ?></title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background: #f5f7fa;
            color: #333;
            line-height: 1.6;
        }
        .container { max-width: 900px; margin: 0 auto; padding: 20px; }
        header {
            text-align: center;
            padding: 30px 0 20px;
        }
        header h1 { font-size: 1.8rem; color: #2c3e50; }
        header p { color: #7f8c8d; margin-top: 5px; }
        .card {
            background: #fff;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.08);
            padding: 24px;
            margin-bottom: 20px;
        }
        .card h2 { font-size: 1.1rem; color: #34495e; margin-bottom: 16px; }
        textarea {
            width: 100%;
            height: 100px;
            border: 1px solid #ddd;
            border-radius: 6px;
            padding: 12px;
            font-size: 16px;
            font-family: inherit;
            resize: vertical;
        }
        textarea:focus { outline: none; border-color: #3498db; box-shadow: 0 0 0 2px rgba(52,152,219,0.2); }
        .controls {
            display: flex;
            gap: 12px;
            align-items: center;
            margin-top: 12px;
            flex-wrap: wrap;
        }
        select {
            padding: 8px 12px;
            border: 1px solid #ddd;
            border-radius: 6px;
            font-size: 14px;
            background: #fff;
        }
        button {
            padding: 10px 24px;
            background: #3498db;
            color: #fff;
            border: none;
            border-radius: 6px;
            font-size: 14px;
            cursor: pointer;
            transition: background 0.2s;
        }
        button:hover { background: #2980b9; }
        button:disabled { background: #bdc3c7; cursor: not-allowed; }
        .status { font-size: 13px; color: #7f8c8d; margin-left: auto; }
        table {
            width: 100%;
            border-collapse: collapse;
            font-size: 14px;
        }
        th {
            text-align: left;
            padding: 10px 12px;
            background: #f8f9fa;
            border-bottom: 2px solid #dee2e6;
            font-weight: 600;
            color: #495057;
        }
        td {
            padding: 8px 12px;
            border-bottom: 1px solid #eee;
        }
        tr:hover td { background: #f8f9fa; }
        .surface { font-weight: 600; font-size: 15px; }
        .unknown { color: #e74c3c; }
        .details { color: #7f8c8d; font-size: 13px; }
        .empty { text-align: center; color: #aaa; padding: 40px; }
        .error { color: #e74c3c; padding: 12px; background: #fdf0ef; border-radius: 6px; }
        @media (max-width: 600px) {
            .controls { flex-direction: column; align-items: stretch; }
            .status { margin-left: 0; margin-top: 8px; }
        }
    </style>
</head>
<body>
<div class="container">
    <header>
        <h1>Lindera PHP v<?= htmlspecialchars($version) ?></h1>
        <p>PHP Morphological Analysis Web Application</p>
    </header>

    <div class="card">
        <h2>Input</h2>
        <textarea id="text" placeholder="Enter text here (e.g., すもももももももものうち)">関西国際空港限定トートバッグを東京スカイツリーの最寄り駅であるとうきょうスカイツリー駅で買う</textarea>
        <div class="controls">
            <select id="mode">
                <option value="normal">Normal</option>
                <option value="decompose">Decompose</option>
            </select>
            <button id="analyze" onclick="analyze()">Run Morphological Analysis</button>
            <span class="status" id="status"></span>
        </div>
    </div>

    <div class="card">
        <h2>Results</h2>
        <div id="results"><p class="empty">Enter text above and click "Run Morphological Analysis"</p></div>
    </div>
</div>

<script>
async function analyze() {
    const text = document.getElementById('text').value.trim();
    const mode = document.getElementById('mode').value;
    const btn = document.getElementById('analyze');
    const status = document.getElementById('status');
    const results = document.getElementById('results');

    if (!text) {
        results.innerHTML = '<p class="error">Please enter some text.</p>';
        return;
    }

    btn.disabled = true;
    status.textContent = 'Analyzing...';

    try {
        const res = await fetch(window.location.href, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ text, mode })
        });
        const data = await res.json();

        if (data.error) {
            results.innerHTML = `<p class="error">${escapeHtml(data.error)}</p>`;
            status.textContent = 'Error';
            return;
        }

        if (data.tokens.length === 0) {
            results.innerHTML = '<p class="empty">No tokens found.</p>';
            status.textContent = 'Done (0 tokens)';
            return;
        }

        let html = '<table><thead><tr><th>Surface</th><th>Position</th><th>Details</th></tr></thead><tbody>';
        for (const t of data.tokens) {
            const cls = t.is_unknown ? ' unknown' : '';
            const details = t.details ? t.details.join(', ') : '';
            html += `<tr>
                <td class="surface${cls}">${escapeHtml(t.surface)}${t.is_unknown ? ' <small>(UNK)</small>' : ''}</td>
                <td>${t.position}</td>
                <td class="details">${escapeHtml(details)}</td>
            </tr>`;
        }
        html += '</tbody></table>';
        results.innerHTML = html;
        status.textContent = `Done (${data.tokens.length} tokens)`;
    } catch (e) {
        results.innerHTML = `<p class="error">${escapeHtml(e.message)}</p>`;
        status.textContent = 'Error';
    } finally {
        btn.disabled = false;
    }
}

function escapeHtml(s) {
    const div = document.createElement('div');
    div.textContent = s;
    return div.innerHTML;
}

// Ctrl+Enter to analyze
document.getElementById('text').addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') analyze();
});
</script>
</body>
</html>
