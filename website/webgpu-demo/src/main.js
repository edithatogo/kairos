const canvas = document.querySelector("#viewport");
const ctx = canvas.getContext("2d");
const agentCountSelect = document.querySelector("#agent-count");
const backendSelect = document.querySelector("#backend");
const resetButton = document.querySelector("#reset");
const metricBackend = document.querySelector("#metric-backend");
const metricAgents = document.querySelector("#metric-agents");
const metricFps = document.querySelector("#metric-fps");
const metricCompute = document.querySelector("#metric-compute");
const metricRender = document.querySelector("#metric-render");

let agents = [];
let lastFrame = performance.now();

function hasWebGpu() {
  return Boolean(navigator.gpu);
}

function resetAgents() {
  const count = Number(agentCountSelect.value);
  agents = Array.from({ length: count }, (_, index) => ({
    x: (index * 37) % canvas.width,
    y: (index * 19) % canvas.height,
    vx: ((index % 11) - 5) * 0.08,
    vy: ((index % 7) - 3) * 0.08
  }));
  metricAgents.textContent = String(count);
}

function stepCpu(dt) {
  for (let index = 0; index < agents.length; index += 1) {
    const agent = agents[index];
    agent.x = (agent.x + agent.vx * dt + canvas.width) % canvas.width;
    agent.y = (agent.y + agent.vy * dt + canvas.height) % canvas.height;
  }
}

function render() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = "#64d2ff";
  const stride = Math.max(1, Math.floor(agents.length / 12000));
  for (let index = 0; index < agents.length; index += stride) {
    const agent = agents[index];
    ctx.fillRect(agent.x, agent.y, 1, 1);
  }
}

function frame(now) {
  const dt = Math.min(32, now - lastFrame);
  lastFrame = now;

  const requestedBackend = backendSelect.value;
  const effectiveBackend = requestedBackend === "webgpu" && hasWebGpu() ? "WebGPU ready" : "CPU fallback";
  metricBackend.textContent = effectiveBackend;

  const computeStart = performance.now();
  stepCpu(dt);
  const computeMs = performance.now() - computeStart;

  const renderStart = performance.now();
  render();
  const renderMs = performance.now() - renderStart;

  metricFps.textContent = String(Math.round(1000 / Math.max(dt, 1)));
  metricCompute.textContent = `${computeMs.toFixed(2)} ms`;
  metricRender.textContent = `${renderMs.toFixed(2)} ms`;

  requestAnimationFrame(frame);
}

agentCountSelect.addEventListener("change", resetAgents);
resetButton.addEventListener("click", resetAgents);
resetAgents();
requestAnimationFrame(frame);
