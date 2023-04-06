const core = globalThis.Deno.core;
const ops = core.ops;

function getEnv(key) {
  return ops.op_get_env(key) ?? undefined;
}

const env = {
  get: getEnv,
};

export { env };
