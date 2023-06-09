const primordials = globalThis.__bootstrap.primordials;
const { ObjectDefineProperties } = primordials;
const core = globalThis.Deno.core;

import { globalScope } from "ext:mashin_core/98_global_scope.js";
import { env } from "ext:mashin_core/30_os.js";
import { errors } from "ext:mashin_core/01_errors.js";
import {
  downloadProvider,
  DynamicProvider,
  DynamicResource,
} from "ext:mashin_core/40_ffi.js";
import DOMException from "ext:deno_web/01_dom_exception.js";
import * as util from "ext:mashin_core/06_util.js";
// console
import * as console from "ext:deno_console/01_console.js";

// Set up global properties
ObjectDefineProperties(globalThis, globalScope);

let hasBootstrapped = false;
function bootstrapMainRuntime(runtimeOptions) {
  if (hasBootstrapped) {
    throw new Error("Main runtime already bootstrapped");
  }
  // remove bootstrapping data from the global scope
  delete globalThis.__bootstrap;
  delete globalThis.bootstrap;
  hasBootstrapped = true;
  // overwrite deno env
  globalThis.Deno.env = env;
  // overwrite deno args
  globalThis.Deno.args = runtimeOptions.args;

  globalThis.Deno.errors = errors;
  globalThis.Deno.build = core.build;
  globalThis.Deno.permissions = {
    request: (perm) => {},
  };

  // only display the console on first run
  ObjectDefineProperties(globalThis, {
    console: util.nonEnumerable(
      runtimeOptions.isFirstRun
        ? new console.Console((msg, level) =>
            core.ops.as__client_print(msg, level > 1)
          )
        : new console.Console((_msg, _level) => {})
    ),
  });
  core.setBuildInfo(runtimeOptions.target);

  core.registerErrorClass("NotFound", errors.NotFound);
  core.registerErrorClass("PermissionDenied", errors.PermissionDenied);
  core.registerErrorClass("ConnectionRefused", errors.ConnectionRefused);
  core.registerErrorClass("ConnectionReset", errors.ConnectionReset);
  core.registerErrorClass("ConnectionAborted", errors.ConnectionAborted);
  core.registerErrorClass("NotConnected", errors.NotConnected);
  core.registerErrorClass("AddrInUse", errors.AddrInUse);
  core.registerErrorClass("AddrNotAvailable", errors.AddrNotAvailable);
  core.registerErrorClass("BrokenPipe", errors.BrokenPipe);
  core.registerErrorClass("AlreadyExists", errors.AlreadyExists);
  core.registerErrorClass("InvalidData", errors.InvalidData);
  core.registerErrorClass("TimedOut", errors.TimedOut);
  core.registerErrorClass("Interrupted", errors.Interrupted);
  core.registerErrorClass("WouldBlock", errors.WouldBlock);
  core.registerErrorClass("WriteZero", errors.WriteZero);
  core.registerErrorClass("UnexpectedEof", errors.UnexpectedEof);
  core.registerErrorClass("BadResource", errors.BadResource);
  core.registerErrorClass("Http", errors.Http);
  core.registerErrorClass("Busy", errors.Busy);
  core.registerErrorClass("NotSupported", errors.NotSupported);
  core.registerErrorBuilder(
    "DOMExceptionOperationError",
    function DOMExceptionOperationError(msg) {
      return new DOMException(msg, "OperationError");
    }
  );
  core.registerErrorBuilder(
    "DOMExceptionQuotaExceededError",
    function DOMExceptionQuotaExceededError(msg) {
      return new DOMException(msg, "QuotaExceededError");
    }
  );
  core.registerErrorBuilder(
    "DOMExceptionNotSupportedError",
    function DOMExceptionNotSupportedError(msg) {
      return new DOMException(msg, "NotSupported");
    }
  );
  core.registerErrorBuilder(
    "DOMExceptionNetworkError",
    function DOMExceptionNetworkError(msg) {
      return new DOMException(msg, "NetworkError");
    }
  );
  core.registerErrorBuilder(
    "DOMExceptionAbortError",
    function DOMExceptionAbortError(msg) {
      return new DOMException(msg, "AbortError");
    }
  );
  core.registerErrorBuilder(
    "DOMExceptionInvalidCharacterError",
    function DOMExceptionInvalidCharacterError(msg) {
      return new DOMException(msg, "InvalidCharacterError");
    }
  );
  core.registerErrorBuilder(
    "DOMExceptionDataError",
    function DOMExceptionDataError(msg) {
      return new DOMException(msg, "DataError");
    }
  );
}

globalThis.bootstrap = {
  mainRuntime: bootstrapMainRuntime,
};

globalThis.__mashin = {
  DynamicProvider,
  DynamicResource,
  downloadProvider,
};
