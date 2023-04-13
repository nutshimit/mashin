import { pathFromURL } from "ext:deno_web/00_infra.js";
const core = globalThis.Deno.core;
const ops = core.ops;

class DynamicProvider {
  constructor(name, path, props) {
    ops.as__runtime__register_provider__allocate({
      name,
      path: pathFromURL(path),
      symbols: {
        new: {
          parameters: ["pointer", "pointer"],
          result: "pointer",
        },
        run: {
          parameters: ["pointer", "pointer"],
          result: "pointer",
        },
        drop: {
          parameters: ["pointer"],
          result: "void",
        },
      },
      props,
    });
  }
}

class DynamicResource {
  #output;
  constructor(urn, config) {
    this.#output = ops.as__runtime__resource_execute({
      urn,
      config,
    });
  }

  output() {
    return this.#output;
  }
}

export { DynamicProvider, DynamicResource };
