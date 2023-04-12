import { Mashin } from "./mod.ts";
import { Inputs, Outputs } from "./output.ts";

export type StepFn = () => Promise<void>;
export type StateRecord = Record<string, string>;

export type ProviderList = [string, number];

declare global {
  namespace Deno {
    namespace core {
      function opAsync<T>(opName: string, ...args: unknown[]): Promise<T>;
    }
  }

  namespace __mashin {
    let rid: number | null;
    let engine: Mashin | null;
    let providers: ProviderList[];
    class DynamicProvider {
      constructor(name: string, path: string);
    }
    class DynamicResource {
      constructor(urn: string, config: Inputs);

      output(): Outputs;
    }
  }
}
