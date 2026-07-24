import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createFromRoot } from "codama";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-js";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.join(scriptDir, "..");
const idlPath = path.join(webDir, "..", "app", "target", "idl", "app.json");

const anchorIdl = JSON.parse(readFileSync(idlPath, "utf-8"));
const codama = createFromRoot(rootNodeFromAnchor(anchorIdl));

await codama.accept(renderVisitor(webDir, { syncPackageJson: false }));
