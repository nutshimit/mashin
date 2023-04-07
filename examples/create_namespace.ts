#!/usr/bin/env mashin run

import { Mashin } from "../sdk/ts/mod.ts";
import * as aws from "../crates/test/bindings/mod.ts";
//import { Provider as Provider2 } from "../crates/test2/bindings/mod.ts";

// init mashin engine
await new Mashin().setup();

// configure aws provider
const provider = await new aws.Provider("aws", {
  accessKey: "AKIAIOSFODNN7EXAMPLE",
}).setup();

const bucket = await new aws.s3.Bucket(
  "test1234atmos001",
  {
    acl: "AAAAAAAAdsafadfadfadsAA",
  },
  {
    provider,
    protect: true,
  }
).create();

console.log(bucket.data);
