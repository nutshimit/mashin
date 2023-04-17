/* -------------------------------------------------------- *\
*                                                          *
*      ███╗░░░███╗░█████╗░░██████╗██╗░░██╗██╗███╗░░██╗     *
*      ████╗░████║██╔══██╗██╔════╝██║░░██║██║████╗░██║     *
*      ██╔████╔██║███████║╚█████╗░███████║██║██╔██╗██║     *
*      ██║╚██╔╝██║██╔══██║░╚═══██╗██╔══██║██║██║╚████║     *
*      ██║░╚═╝░██║██║░░██║██████╔╝██║░░██║██║██║░╚███║     *
*      ╚═╝░░░░░╚═╝╚═╝░░╚═╝╚═════╝░╚═╝░░╚═╝╚═╝╚═╝░░╚══╝     *
*                                         by Nutshimit     *
* -------------------------------------------------------- *
*                                                          *
*   This file is generated automatically by mashin.        *
*   Do not edit manually.                                  *
*                                                          *
\* ---------------------------------------------------------*/

import * as resource from "../../sdk/ts/resource.ts";
import { Inputs, Outputs } from "../../sdk/ts/output.ts";

const url = Deno.env.get("LOCAL_PLUGIN")
  ? "./target/debug/libatmosphere_test.dylib"
  : await globalThis.__mashin.downloadProvider(
      "github", "https://github.com/lemarier/tauri-test/releases/download/v2.0.0/libatmosphere_test.dylib"
    );

export interface BucketOutputs extends Outputs {
  url: string | undefined | null;
  password: string | undefined | null;
  test: string | undefined | null;
};

export class Bucket<T extends Lowercase<string>> extends resource.Resource<
BucketOutputs,
T
> {
    #props: BucketConfig;
    constructor(
        name: resource.ResourceName<T>,
        props: BucketConfig,
        opts: resource.ResourceOptions
    ) {
        super(name, "s3:bucket", props, opts);
        this.#props = props;
    }

    get props() {
        return this.#props;
    }
}

export type ExtraConfig = {
  accessKey: string | undefined | null;
};
export interface BucketConfig extends Inputs {
  acl: string | undefined | null;
  woot: boolean | undefined | null;
}
/**
  * This is my config
  **/
export interface Config extends Inputs {
/**
  * This is a test
  **/
  accessKey: string | undefined | null;
/**
  * This is a another test
  * with multi line
  **/
  test: ExtraConfig | undefined | null;
}

export class Provider extends resource.Provider {
    constructor(name: string, args?: Config) {
      // FIXME: have dynamic provider path for each OS
      super(name, url, args);
    }
}

// 13116358352885281898