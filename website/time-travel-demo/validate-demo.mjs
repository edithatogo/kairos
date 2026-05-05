import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const root = path.dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
  fs.readFileSync(path.join(root, "trace-fixture.json"), "utf8"),
);

if (fixture.schema !== "kairo.ecs.trace.v1") {
  throw new Error(`unexpected trace schema: ${fixture.schema}`);
}

let previousTick = -1;
for (const event of fixture.events) {
  if (!Number.isInteger(event.tick) || event.tick < previousTick) {
    throw new Error("trace fixture ticks must be monotonically increasing integers");
  }
  if (!event.state || typeof event.state["machine.status"] !== "string") {
    throw new Error("trace fixture event missing machine.status state");
  }
  previousTick = event.tick;
}

const elements = new Map();

function element(id) {
  if (!elements.has(id)) {
    elements.set(id, {
      id,
      children: [],
      className: "",
      textContent: "",
      type: "",
      innerHTML: "",
      listeners: {},
      appendChild(child) {
        this.children.push(child);
      },
      addEventListener(type, handler) {
        this.listeners[type] = handler;
      },
    });
  }
  return elements.get(id);
}

const document = {
  querySelector(selector) {
    return element(selector.replace("#", ""));
  },
  createElement(tag) {
    return element(`${tag}-${Math.random()}`);
  },
};

const script = fs.readFileSync(path.join(root, "main.js"), "utf8");
vm.runInNewContext(script, {
  clearInterval,
  console,
  document,
  JSON,
  Math,
  setInterval,
  window: { KAIRO_TRACE_EVENTS: fixture.events },
});

const tick = element("tick").textContent;
const state = JSON.parse(element("state").textContent);
const events = element("events").children;

if (tick !== "tick 0") {
  throw new Error(`expected initial tick 0, got ${tick}`);
}
if (events.length !== fixture.events.length) {
  throw new Error(`expected ${fixture.events.length} event controls, got ${events.length}`);
}
if (state["machine.status"] !== "idle") {
  throw new Error("expected initial state to render machine.status=idle");
}

element("step").listeners.click();
if (element("tick").textContent !== "tick 2") {
  throw new Error("step control did not advance to tick 2");
}

element("back").listeners.click();
if (element("tick").textContent !== "tick 0") {
  throw new Error("back control did not return to tick 0");
}
