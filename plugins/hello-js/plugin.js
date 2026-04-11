/**
 * Rustacle "hello-js" plugin — a JavaScript WASM Component.
 *
 * Proves that the WIT contract is language-neutral:
 * same interface, different language, same host.
 *
 * Implements: rustacle:plugin/module
 * Uses:       rustacle:plugin/host (log)
 */

// The host object is injected by the component model linker.
// We access it via the import namespace.
let callCount = 0;

export const module = {
  /**
   * Return the plugin manifest.
   */
  manifest() {
    return {
      id: "rustacle.hello-js",
      version: "0.1.0",
      capabilities: [],
      subscriptions: [],
      uiContributions: {
        panels: [
          {
            id: "hello-js-panel",
            title: "Hello JS",
            icon: "🟨",
          },
        ],
        paletteEntries: [
          {
            id: "hello-js.greet",
            label: "Hello JS: Greet",
            shortcut: "",
          },
        ],
        settingsSchema: "",
      },
    };
  },

  /**
   * Initialize the plugin.
   */
  init() {
    // host.log("info", "hello-js plugin initialized", new Uint8Array(0));
  },

  /**
   * Handle an event from the event bus.
   */
  onEvent(_topic, _payload) {
    // No-op for this demo plugin
  },

  /**
   * Shut down the plugin.
   */
  shutdown() {
    // host.log("info", "hello-js plugin shutting down", new Uint8Array(0));
  },

  /**
   * Handle a command call.
   */
  call(command, payload) {
    callCount++;

    if (command === "greet") {
      const input = JSON.parse(new TextDecoder().decode(new Uint8Array(payload)));
      const name = input.name || "World";
      const response = {
        message: `Hello from JavaScript, ${name}!`,
        language: "JavaScript",
        plugin_id: "rustacle.hello-js",
        call_count: callCount,
        timestamp: Date.now(),
      };
      return new TextEncoder().encode(JSON.stringify(response));
    }

    if (command === "ping") {
      const response = {
        message: "pong from JS plugin",
        language: "JavaScript",
        plugin_id: "rustacle.hello-js",
        call_count: callCount,
        timestamp: Date.now(),
      };
      return new TextEncoder().encode(JSON.stringify(response));
    }

    if (command === "info") {
      const response = {
        runtime: "JavaScript (componentize-js)",
        wit_version: "rustacle:plugin@0.1.0",
        description: "Proves WASM Component Model is language-neutral",
        supported_commands: ["greet", "ping", "info"],
      };
      return new TextEncoder().encode(JSON.stringify(response));
    }

    throw { tag: "invalid-input", val: `unknown command: ${command}` };
  },
};
