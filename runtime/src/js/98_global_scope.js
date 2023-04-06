// console
import * as console from "ext:deno_console/02_console.js";
// fetch
import * as headers from "ext:deno_fetch/20_headers.js";
import * as formData from "ext:deno_fetch/21_formdata.js";
import * as request from "ext:deno_fetch/23_request.js";
import * as response from "ext:deno_fetch/23_response.js";
import * as fetch from "ext:deno_fetch/26_fetch.js";
// utils
import * as util from "ext:mashin_core/06_util.js";
// websocket
import * as webSocket from "ext:deno_websocket/01_websocket.js";
// web
import * as event from "ext:deno_web/02_event.js";
import * as timers from "ext:deno_web/02_timers.js";
import * as base64 from "ext:deno_web/05_base64.js";
import * as encoding from "ext:deno_web/08_text_encoding.js";
import * as url from "ext:deno_url/00_url.js";
import * as urlPattern from "ext:deno_url/01_urlpattern.js";

const core = globalThis.Deno.core;

const globalScope = {
  console: util.nonEnumerable(
    new console.Console((msg, level) => core.print(msg, level > 1))
  ),
  ErrorEvent: util.nonEnumerable(event.ErrorEvent),
  Event: util.nonEnumerable(event.Event),
  EventTarget: util.nonEnumerable(event.EventTarget),
  Headers: util.nonEnumerable(headers.Headers),
  Request: util.nonEnumerable(request.Request),
  Response: util.nonEnumerable(response.Response),
  FormData: util.nonEnumerable(formData.FormData),
  fetch: util.nonEnumerable(fetch.fetch),
  WebSocket: util.nonEnumerable(webSocket.WebSocket),
  setInterval: util.writable(timers.setInterval),
  setTimeout: util.writable(timers.setTimeout),
  atob: util.writable(base64.atob),
  btoa: util.writable(base64.btoa),
  clearInterval: util.writable(timers.clearInterval),
  clearTimeout: util.writable(timers.clearTimeout),
  TextDecoder: util.nonEnumerable(encoding.TextDecoder),
  TextEncoder: util.nonEnumerable(encoding.TextEncoder),
  TextDecoderStream: util.nonEnumerable(encoding.TextDecoderStream),
  TextEncoderStream: util.nonEnumerable(encoding.TextEncoderStream),
  URL: util.nonEnumerable(url.URL),
  URLPattern: util.nonEnumerable(urlPattern.URLPattern),
  URLSearchParams: util.nonEnumerable(url.URLSearchParams),
};

export { globalScope };
