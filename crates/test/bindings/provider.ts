import * as resource from "../../../sdk/ts/resource.ts";
import { Inputs, Input, Output } from "../../../sdk/ts/output.ts";

export class Provider extends resource.Provider {
  constructor(name: string, args?: ProviderArgs) {
    const resourceInputs: Inputs = {};

    resourceInputs["accessKey"] = args?.accessKey;

    // FIXME: have dynamic provider path for each OS
    super(name, "./target/debug/libtest.dylib", resourceInputs);
  }
}

export interface ProviderArgs {
  accessKey?: Input<string>;
}
