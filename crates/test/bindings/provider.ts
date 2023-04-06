import * as resource from "../../../sdk/ts/resource.ts";
import { Inputs, Input, Output } from "../../../sdk/ts/output.ts";
import { lib } from "./ffi.ts";

export class Provider extends resource.Provider {
  constructor(name: string, args?: ProviderArgs) {
    const resourceInputs: Inputs = {};

    resourceInputs["accessKey"] = args?.accessKey;

    super(lib.rid(), name, resourceInputs);
  }
}

export interface ProviderArgs {
  accessKey?: Input<string>;
}
