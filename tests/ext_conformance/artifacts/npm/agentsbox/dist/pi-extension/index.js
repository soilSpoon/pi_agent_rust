import { realpath } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const realHere = await realpath(here);
const targetUrl = pathToFileURL(join(realHere, "..", "pi.js")).href;
const mod = await import(targetUrl);

export default mod.default;
