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
  #rid: number;

  constructor(rid: number, name: string, props: Inputs = {}) {
    super(name, props);
    this.#rid = rid;
  }

  get rid() {
    return this.#rid;
  }

  async setup() {
    await Deno.core.opAsync(
      "as__runtime__register_provider__allocate",
      this.#rid,
      this.name
    );

    globalThis.__mashin.providers.push([this.name, this.#rid]);

    return this;
  }
}

export abstract class Resource<
  O extends Outputs,
  T extends Lowercase<string>
> extends Base {
  #urn: URN;
  #opts: ResourceOptions;
  #output: O | undefined;
  #module: Lowercase<string>;
  #resource_type: Lowercase<string>;

  constructor(
    name: ResourceName<T>,
    module: Lowercase<string>,
    resource_type: Lowercase<string>,
    props: Inputs,
    opts: ResourceOptions
  ) {
    super(name, props);
    // urn:provider:aws:s3:mysuper_bucket
    this.#module = module;
    this.#resource_type = resource_type;
    this.#opts = opts;
    this.#urn = `urn:provider:${opts.provider.name}:${this.#module}:${
      this.#resource_type
    }?=${name}`;
  }

  async create() {
    this.#output = await Deno.core.opAsync(
      "as__runtime__provider__dry_run",
      globalThis.__mashin.rid,
      this.#opts.provider.rid,
      this.#urn,
      this.props
    );
    return this;
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
  get module() {
    return this.#module;
  }
}
