#!/usr/bin/env mashin run
import * as aws from "../crates/test/bindings/mod.ts";

// init mashin engine
//await new Mashin().setup();

// configure aws provider
const provider = new aws.Provider("aws", {
  accessKey: "AKIAIOSFODNN7EXAMPLE",
});

const bucket = new aws.s3.Bucket(
  "test1234atmos001",
  {
    acl: "public-read",
  },
  {
    provider,
    protect: true,
  }
);

const bucket2 = new aws.s3.Bucket(
  "yayabitch",
  {
    acl: "public-read",
  },
  {
    provider,
    protect: true,
  }
);

//console.log(bucket.data);
