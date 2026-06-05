/**
 * Entry point. Mounts the Solid app to `#root`.
 */

import { render } from "solid-js/web";
import { App } from "./App";
import { initStores } from "./lib/store";

import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/monaco-theme.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("#root element not found");
}

// Initialize stores, then mount the app. We do this before render so
// the first paint already has data.
void initStores().finally(() => {
  render(() => <App />, root);
});
