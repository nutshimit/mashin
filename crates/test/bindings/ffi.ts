import { dlopen, download } from "./deps.ts";
const version = "0.1.0";

//const cache = Deno.env.get("PLUGIN_URL") === undefined ? "use" : "reloadAll";
const cache = "reloadAll";
const url = "./target/debug/libtest.dylib";

export const lib = Deno.dlopen(url, {
  new: {
    parameters: [],
    result: "pointer",
  },
  run: {
    parameters: [
      "pointer",
      "pointer",
      "pointer",
      "pointer",
      "pointer",
      "pointer",
    ],
    result: "pointer",
  },
  drop: {
    parameters: ["pointer"],
    result: "void",
  },
} as const);

/*
export const lib = await dlopen(
  {
    name: "test",
    url,
    cache,
  },
  {
    new: {
      parameters: [],
      result: "pointer",
    },
    run: {
      parameters: ["pointer"],
      result: "void",
    },
  } as const
);
*/
