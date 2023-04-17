/// <reference types="./mashin.d.ts" />

import { Input, Inputs, Output, Outputs } from "./output.ts";

export type ResourceNameBlacklist =
  | " "
  | "-"
  | "$"
  | "."
  | "/"
  | "\\"
  | "*"
  | ":"
  | "?"
  | "_";
export type ResourceName<T extends string> =
  T extends `${infer Prefix}${ResourceNameBlacklist}${infer Suffix}`
    ? `Invalid character in resource name: '${T}'`
    : T;

export const NIL_UUID = "00000000-0000-0000-0000-000000000000";
export type ID = string;
export type URN = string;

export abstract class Base {
  #name: string;
  #props: Inputs;
  constructor(name: string, props: Inputs) {
    this.#name = name;
    this.#props = props;
  }
  get name() {
    return this.#name;
  }
  get props() {
    return this.#props;
  }
}

export interface ResourceOptions {
  //parent?: Resource;
  protect?: boolean;
  provider: Provider;
  deleteBeforeReplace?: boolean;
}

export abstract class Provider extends Base {
  constructor(name: string, path: string, props: Inputs = {}) {
    super(name, props);
    new __mashin.DynamicProvider(name, path, props);
  }
}

export abstract class Resource<
  O extends Outputs,
  T extends Lowercase<string>
> extends Base {
  #urn: URN;
  #opts: ResourceOptions;
  #output: O | undefined;

  #resource_type: Lowercase<string>;

  constructor(
    name: ResourceName<T>,

    resource_type: Lowercase<string>,
    props: Inputs,
    opts: ResourceOptions
  ) {
    super(name, props);
    // urn:provider:aws:s3:mysuper_bucket
    this.#resource_type = resource_type;
    this.#opts = opts;
    this.#urn = `urn:provider:${opts.provider.name}:${
      this.#resource_type
    }?=${name}`;

    this.#output = new __mashin.DynamicResource(this.#urn, props).output() as O;
  }

  get opts() {
    return this.#opts;
  }
  get urn() {
    return this.#urn;
  }
  get data() {
    return this.#output;
  }
}
