import __wbg_init, { TokenizerBuilder, getVersion } from '../../pkg/lindera_wasm.js';

// Initialize the tokenizer
let tokenizer = null;

// Initialize WASM module first
__wbg_init().then(() => {
    // Show the version in the title
    try {
        const version = getVersion();
        document.title = `Lindera WASM v${version}`;
        document.getElementById('title').textContent = `Lindera WASM v${version}`;
    } catch (e) {
        console.error("Failed to get version:", e);
    }

    try {
        // Create a TokenizerBuilder instance
        let builder = new TokenizerBuilder();
        // Set the dictionary to "ipadic" (Japanese)
        // You can also use "ko-dic" (Korean) or "cc-cedict" (Chinese) as the dictionary
        builder.setDictionary("embedded://ipadic");

        // Set the tokenizer mode to "normal"
        // You can also use "decompose" for decomposing the compound words into their components
        builder.setMode("normal");

        // Set to not keep whitespace tokens
        // You can set it to true if you want to keep whitespace tokens
        builder.setKeepWhitespace(false);

        // Append character filters
        builder.appendCharacterFilter("unicode_normalize", { "kind": "nfkc" });

        // Append token filters
        builder.appendTokenFilter("lowercase");
        builder.appendTokenFilter("japanese_compound_word", {
            "kind": "ipadic",
            "tags": [
                "名詞,数"
            ],
            "new_tag": "名詞,数"
        });
        builder.appendTokenFilter("japanese_number", { "tags": ["名詞,数"] });

        // Build the Tokenizer instance
        tokenizer = builder.build();

        console.log("Tokenizer is ready.");
    } catch (e) {
        // Handle the error
        console.error("Failed to create Tokenizer:", e);
    }
}).catch(e => {
    console.error("Failed to initialize WASM module:", e);
});

// Add an event listener to the "runButton" element
document.getElementById('runButton').addEventListener('click', () => {
    // If the tokenizer is not initialized yet, display an error message
    if (!tokenizer) {
        console.error("Tokenizer is not initialized yet.");
        return;
    }

    // Get the input text from the "inputText" element
    const inputText = document.getElementById('inputText').value;

    // Tokenize the input text
    const tokens = tokenizer.tokenize(inputText);

    // Get the "resultList" element
    const resultList = document.getElementById('resultList');

    // Clear the previous results
    resultList.innerHTML = '';

    // Display the tokens
    console.log('All tokens:', tokens); // Log the entire tokens array

    tokens.forEach((token, index) => {
        const li = document.createElement('li');
        const pre = document.createElement('pre');

        console.log(`Token ${index}:`, token); // Log each individual token object

        pre.textContent = JSON.stringify(token, null, 2);
        li.appendChild(pre);

        resultList.appendChild(li);
    });
});
