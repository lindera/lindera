// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="index.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded "><a href="architecture.html"><strong aria-hidden="true">2.</strong> Architecture</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded "><a href="getting_started.html"><strong aria-hidden="true">3.</strong> Getting Started</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="getting_started/installation.html"><strong aria-hidden="true">3.1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="getting_started/quickstart.html"><strong aria-hidden="true">3.2.</strong> Quick Start</a></li><li class="chapter-item expanded "><a href="getting_started/examples.html"><strong aria-hidden="true">3.3.</strong> Examples</a></li></ol></li><li class="chapter-item expanded "><li class="spacer"></li><li class="chapter-item expanded "><a href="concepts.html"><strong aria-hidden="true">4.</strong> Core Concepts</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="concepts/morphological_analysis.html"><strong aria-hidden="true">4.1.</strong> Morphological Analysis</a></li><li class="chapter-item expanded "><a href="concepts/dictionaries.html"><strong aria-hidden="true">4.2.</strong> Dictionaries</a></li><li class="chapter-item expanded "><a href="concepts/tokenization.html"><strong aria-hidden="true">4.3.</strong> Tokenization</a></li><li class="chapter-item expanded "><a href="concepts/user_dictionary.html"><strong aria-hidden="true">4.4.</strong> User Dictionary</a></li><li class="chapter-item expanded "><a href="concepts/character_filters.html"><strong aria-hidden="true">4.5.</strong> Character Filters</a></li></ol></li><li class="chapter-item expanded "><li class="spacer"></li><li class="chapter-item expanded "><a href="lindera-crf.html"><strong aria-hidden="true">5.</strong> Lindera CRF</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-crf/architecture.html"><strong aria-hidden="true">5.1.</strong> Architecture</a></li><li class="chapter-item expanded "><a href="lindera-crf/api_reference.html"><strong aria-hidden="true">5.2.</strong> API Reference</a></li></ol></li><li class="chapter-item expanded "><a href="lindera-dictionary.html"><strong aria-hidden="true">6.</strong> Lindera Dictionary</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-dictionary/architecture.html"><strong aria-hidden="true">6.1.</strong> Architecture</a></li><li class="chapter-item expanded "><a href="lindera-dictionary/api_reference.html"><strong aria-hidden="true">6.2.</strong> API Reference</a></li></ol></li><li class="chapter-item expanded "><a href="lindera.html"><strong aria-hidden="true">7.</strong> Lindera Library</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera/configuration.html"><strong aria-hidden="true">7.1.</strong> Configuration</a></li><li class="chapter-item expanded "><a href="lindera/segmenter.html"><strong aria-hidden="true">7.2.</strong> Segmenter</a></li><li class="chapter-item expanded "><a href="lindera/token_filters.html"><strong aria-hidden="true">7.3.</strong> Token Filters</a></li><li class="chapter-item expanded "><a href="lindera/error_handling.html"><strong aria-hidden="true">7.4.</strong> Error Handling</a></li><li class="chapter-item expanded "><a href="lindera/api_reference.html"><strong aria-hidden="true">7.5.</strong> API Reference</a></li></ol></li><li class="chapter-item expanded "><a href="lindera-cli.html"><strong aria-hidden="true">8.</strong> Lindera CLI</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-cli/installation.html"><strong aria-hidden="true">8.1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="lindera-cli/commands.html"><strong aria-hidden="true">8.2.</strong> Commands</a></li><li class="chapter-item expanded "><a href="lindera-cli/tutorial.html"><strong aria-hidden="true">8.3.</strong> Tutorial</a></li></ol></li><li class="chapter-item expanded "><a href="lindera-python.html"><strong aria-hidden="true">9.</strong> Lindera Python</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-python/installation.html"><strong aria-hidden="true">9.1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="lindera-python/quickstart.html"><strong aria-hidden="true">9.2.</strong> Quick Start</a></li><li class="chapter-item expanded "><a href="lindera-python/tokenizer_api.html"><strong aria-hidden="true">9.3.</strong> Tokenizer API</a></li><li class="chapter-item expanded "><a href="lindera-python/dictionary_management.html"><strong aria-hidden="true">9.4.</strong> Dictionary Management</a></li><li class="chapter-item expanded "><a href="lindera-python/text_processing_pipeline.html"><strong aria-hidden="true">9.5.</strong> Text Processing Pipeline</a></li><li class="chapter-item expanded "><a href="lindera-python/training.html"><strong aria-hidden="true">9.6.</strong> Training</a></li></ol></li><li class="chapter-item expanded "><a href="lindera-wasm.html"><strong aria-hidden="true">10.</strong> Lindera WASM</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-wasm/installation.html"><strong aria-hidden="true">10.1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="lindera-wasm/quickstart.html"><strong aria-hidden="true">10.2.</strong> Quick Start</a></li><li class="chapter-item expanded "><a href="lindera-wasm/tokenizer_api.html"><strong aria-hidden="true">10.3.</strong> Tokenizer API</a></li><li class="chapter-item expanded "><a href="lindera-wasm/dictionary_management.html"><strong aria-hidden="true">10.4.</strong> Dictionary Management</a></li><li class="chapter-item expanded "><a href="lindera-wasm/browser_usage.html"><strong aria-hidden="true">10.5.</strong> Browser Usage</a></li><li class="chapter-item expanded "><a href="lindera-wasm/nodejs_usage.html"><strong aria-hidden="true">10.6.</strong> Node.js Usage</a></li></ol></li><li class="chapter-item expanded "><li class="spacer"></li><li class="chapter-item expanded "><a href="lindera-ipadic.html"><strong aria-hidden="true">11.</strong> Lindera IPADIC</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-ipadic/dictionary_format.html"><strong aria-hidden="true">11.1.</strong> Dictionary Format</a></li><li class="chapter-item expanded "><a href="lindera-ipadic/build.html"><strong aria-hidden="true">11.2.</strong> Build</a></li><li class="chapter-item expanded "><a href="lindera-ipadic/examples.html"><strong aria-hidden="true">11.3.</strong> Examples</a></li></ol></li><li class="chapter-item expanded "><a href="lindera-ipadic-neologd.html"><strong aria-hidden="true">12.</strong> Lindera IPADIC NEologd</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-ipadic-neologd/dictionary_format.html"><strong aria-hidden="true">12.1.</strong> Dictionary Format</a></li><li class="chapter-item expanded "><a href="lindera-ipadic-neologd/build.html"><strong aria-hidden="true">12.2.</strong> Build</a></li><li class="chapter-item expanded "><a href="lindera-ipadic-neologd/examples.html"><strong aria-hidden="true">12.3.</strong> Examples</a></li></ol></li><li class="chapter-item expanded "><a href="lindera-unidic.html"><strong aria-hidden="true">13.</strong> Lindera UniDic</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-unidic/dictionary_format.html"><strong aria-hidden="true">13.1.</strong> Dictionary Format</a></li><li class="chapter-item expanded "><a href="lindera-unidic/build.html"><strong aria-hidden="true">13.2.</strong> Build</a></li><li class="chapter-item expanded "><a href="lindera-unidic/examples.html"><strong aria-hidden="true">13.3.</strong> Examples</a></li></ol></li><li class="chapter-item expanded "><a href="lindera-ko-dic.html"><strong aria-hidden="true">14.</strong> Lindera ko-dic</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-ko-dic/dictionary_format.html"><strong aria-hidden="true">14.1.</strong> Dictionary Format</a></li><li class="chapter-item expanded "><a href="lindera-ko-dic/build.html"><strong aria-hidden="true">14.2.</strong> Build</a></li><li class="chapter-item expanded "><a href="lindera-ko-dic/examples.html"><strong aria-hidden="true">14.3.</strong> Examples</a></li></ol></li><li class="chapter-item expanded "><a href="lindera-cc-cedict.html"><strong aria-hidden="true">15.</strong> Lindera CC-CEDICT</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-cc-cedict/dictionary_format.html"><strong aria-hidden="true">15.1.</strong> Dictionary Format</a></li><li class="chapter-item expanded "><a href="lindera-cc-cedict/build.html"><strong aria-hidden="true">15.2.</strong> Build</a></li><li class="chapter-item expanded "><a href="lindera-cc-cedict/examples.html"><strong aria-hidden="true">15.3.</strong> Examples</a></li></ol></li><li class="chapter-item expanded "><a href="lindera-jieba.html"><strong aria-hidden="true">16.</strong> Lindera Jieba</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="lindera-jieba/dictionary_format.html"><strong aria-hidden="true">16.1.</strong> Dictionary Format</a></li><li class="chapter-item expanded "><a href="lindera-jieba/build.html"><strong aria-hidden="true">16.2.</strong> Build</a></li><li class="chapter-item expanded "><a href="lindera-jieba/examples.html"><strong aria-hidden="true">16.3.</strong> Examples</a></li></ol></li><li class="chapter-item expanded "><li class="spacer"></li><li class="chapter-item expanded "><a href="development.html"><strong aria-hidden="true">17.</strong> Development Guide</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="development/build_and_test.html"><strong aria-hidden="true">17.1.</strong> Build &amp; Test</a></li><li class="chapter-item expanded "><a href="development/feature_flags.html"><strong aria-hidden="true">17.2.</strong> Feature Flags</a></li><li class="chapter-item expanded "><a href="development/project_structure.html"><strong aria-hidden="true">17.3.</strong> Project Structure</a></li><li class="chapter-item expanded "><a href="development/training_pipeline.html"><strong aria-hidden="true">17.4.</strong> Training Pipeline</a></li><li class="chapter-item expanded "><a href="development/contributing.html"><strong aria-hidden="true">17.5.</strong> Contributing</a></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
