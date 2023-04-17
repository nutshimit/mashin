import * as resource from "../../../sdk/ts/resource.ts";
import { Inputs } from "../../../sdk/ts/output.ts";

export class Provider extends resource.Provider {
  constructor(name: string, args?: ProviderArgs) {
    // FIXME: have dynamic provider path for each OS
    super(name, "./target/debug/libtest.dylib", args);
  }
}

export interface ProviderArgs extends Inputs {
  accessKey?: string;
}
