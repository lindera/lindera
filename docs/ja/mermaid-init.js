document.addEventListener("DOMContentLoaded", function () {
    // Transform <pre><code class="language-mermaid">...</code></pre> into <div class="mermaid">...</div>
    const codes = document.querySelectorAll("pre code.language-mermaid");
    codes.forEach(code => {
        const pre = code.parentElement;
        if (pre && pre.tagName === 'PRE') {
            const div = document.createElement("div");
            div.className = "mermaid";
            div.textContent = code.textContent;
            pre.replaceWith(div);
        }
    });

    // Initialize mermaid
    mermaid.initialize({ startOnLoad: true });
});
