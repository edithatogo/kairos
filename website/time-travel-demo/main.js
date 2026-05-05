const fallbackEvents = [
  { tick: 0, state: { "machine.status": "idle" } },
  { tick: 2, state: { "machine.status": "queued" } },
  { tick: 4, state: { "machine.status": "busy" } },
  { tick: 8, state: { "machine.status": "complete" } },
];
const events = window.KAIRO_TRACE_EVENTS || fallbackEvents;

let cursor = 0;
let timer = null;

const eventsEl = document.querySelector("#events");
const stateEl = document.querySelector("#state");
const tickEl = document.querySelector("#tick");

function render() {
  eventsEl.innerHTML = "";
  events.forEach((event, index) => {
    const dot = document.createElement("button");
    dot.type = "button";
    dot.className = `event${index === cursor ? " active" : ""}`;
    dot.textContent = `t=${event.tick}`;
    dot.addEventListener("click", () => {
      cursor = index;
      render();
    });
    eventsEl.appendChild(dot);
  });
  tickEl.textContent = `tick ${events[cursor].tick}`;
  stateEl.textContent = JSON.stringify(events[cursor].state, null, 2);
}

document.querySelector("#back").addEventListener("click", () => {
  cursor = Math.max(0, cursor - 1);
  render();
});

document.querySelector("#step").addEventListener("click", () => {
  cursor = Math.min(events.length - 1, cursor + 1);
  render();
});

document.querySelector("#play").addEventListener("click", (event) => {
  if (timer) {
    clearInterval(timer);
    timer = null;
    event.currentTarget.textContent = "Play";
    return;
  }
  event.currentTarget.textContent = "Pause";
  timer = setInterval(() => {
    cursor = cursor + 1 >= events.length ? 0 : cursor + 1;
    render();
  }, 700);
});

render();
