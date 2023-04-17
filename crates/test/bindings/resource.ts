import * as resource from "../../../sdk/ts/resource.ts";
import { Inputs, Input, Output, Outputs } from "../../../sdk/ts/output.ts";

export interface BucketInputs extends Inputs {
  acl?: Input<
    "private" | "public-read" | "public-read-write" | "aws-exec-read"
  >;
}

export interface BucketOutputs extends Outputs {
  woot: Output<number>;
}

class Bucket<T extends Lowercase<string>> extends resource.Resource<
  BucketOutputs,
  T
> {
  #props: BucketInputs;
  constructor(
    name: resource.ResourceName<T>,
    props: BucketInputs,
    opts: resource.ResourceOptions
  ) {
    super(name, "s3", "bucket", props, opts);
    this.#props = props;
  }

  get props() {
    return this.#props;
  }
}

export const s3 = { Bucket };
