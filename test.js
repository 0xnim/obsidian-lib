import init, { WasmObbyArchive } from "./pkg/obsidian_lib.js";
import { readFileSync } from "fs";

async function main() {
    try {
        // Load the .obby file from the specified directory
        const buffer = readFileSync("./test_dir/ObsidianPlugin.obby");
        console.log("Buffer loaded successfully:", buffer);

        // Initialize the WASM module
        const wasm = await init();
        console.log("WASM module initialized.");

        // Create a new archive instance with the buffer
        const archive = new WasmObbyArchive(buffer);
        console.log("WasmObbyArchive instance created.");

        // List entries in the archive
        const entries = archive.list_entries();
        console.log("Archive entries:", entries);

        // Extract and log the plugin JSON
        const pluginJson = archive.extract_plugin_json();
        console.log("Extracted Plugin JSON:", pluginJson);

    } catch (error) {
        console.error("An error occurred:", error);
    }
}

// Run the main function
main();
