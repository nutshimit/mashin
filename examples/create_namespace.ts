#!/usr/bin/env mashin run
import * as aws from "../crates/test/mod.ts";

// configure aws provider
const provider = new aws.Provider("aws", {
  accessKey: "test",
  test: null,
});

const bucket = new aws.Bucket(
  "test1234atmos001",
  {
    acl: "public-read",
    woot: true,
  },
  {
    provider,
    protect: true,
  }
);

const bucket2 = new aws.Bucket(
  "test1234atmos1000",
  {
    acl: "public-read",
    woot: true,
  },
  {
    provider,
    protect: true,
  }
);

console.log(bucket.data);
