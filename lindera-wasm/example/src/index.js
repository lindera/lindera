import __wbg_init, { TokenizerBuilder, Tokenizer, Mode, load_dictionary, get_version } from '../../pkg/lindera_wasm.js';

// Initialize the tokenizer
let tokenizer = null;

// Initialize WASM module first
__wbg_init().then(() => {
    // Show the version in the title
    try {
        const version = get_version();
        document.title = `Lindera WASM v${version}`;
        document.getElementById('title').textContent = `Lindera WASM v${version}`;
    } catch (e) {
        console.error("Failed to get version:", e);
    }

    try {
        // Option 1: Using TokenizerBuilder (WASM style, snake_case is also supported)
        let builder = new TokenizerBuilder();
        builder.set_dictionary("embedded://ipadic");
        builder.set_mode("normal");
        tokenizer = builder.build();

        // Option 2: Using load_dictionary and Tokenizer constructor (Python style)
        /*
        const dict = load_dictionary("embedded://ipadic");
        tokenizer = new Tokenizer(dict, "normal");
        */

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

    // Create table for results
    const table = document.createElement('table');
    table.className = 'token-table';
    table.innerHTML = `
        <thead>
            <tr>
                <th>Surface</th>
                <th>Position</th>
                <th>Details</th>
            </tr>
        </thead>
        <tbody></tbody>
    `;
    const tbody = table.querySelector('tbody');

    tokens.forEach((token, index) => {
        const tr = document.createElement('tr');

        // Accessing properties directly from Token object
        const surface = token.surface;
        const position = token.position;
        // Using getDetail(index) method
        const detail = token.getDetail(0) || '*';
        const allDetails = token.details.join(', ');

        tr.innerHTML = `
            <td><strong>${surface}</strong></td>
            <td>${position}</td>
            <td><small>${allDetails}</small></td>
        `;
        tbody.appendChild(tr);
    });

    resultList.appendChild(table);
});
