import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Provider } from "react-redux";

import App from "./App";
import "./app.css";
import { store } from "./store";

const root = document.getElementById("root");

if (root === null) {
  throw new Error("missing root element");
}

createRoot(root).render(
  <StrictMode>
    <Provider store={store}>
      <App />
    </Provider>
  </StrictMode>,
);
