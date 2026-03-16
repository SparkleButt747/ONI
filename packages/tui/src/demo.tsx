import React from "react";
import { render } from "ink";
import { App } from "./app.js";

// Clear terminal before rendering
process.stdout.write("\x1b[2J\x1b[3J\x1b[H");

render(<App createDispatch={undefined} />, { exitOnCtrlC: true });
