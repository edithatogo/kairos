(async function bootPlayground() {
  const response = await fetch("headless-snapshot.json");
  const fixture = await response.json();

  const frame = fixture.frame;
  const summary = fixture.expectedSummary;
  const canvas = document.querySelector("#snapshot-canvas");
  const context = canvas.getContext("2d");

  document.querySelector("#tick-value").textContent = String(frame.atTicks);
  document.querySelector("#entity-count").textContent = String(summary.entityCount);
  document.querySelector("#bounds-value").textContent =
    `${summary.bounds.minXMilli},${summary.bounds.minYMilli} to ` +
    `${summary.bounds.maxXMilli},${summary.bounds.maxYMilli} milli`;
  document.querySelector("#fixture-path").textContent = "website/playground/headless-snapshot.json";
  document.querySelector("#source-path").textContent = fixture.sourceExamplePath;
  document.querySelector("#claim-boundary").textContent = fixture.claimBoundary;

  const list = document.querySelector("#entity-list");
  list.innerHTML = "";
  for (const entity of frame.entities) {
    const item = document.createElement("li");
    item.textContent = `${entity.label} (${entity.xMilli}, ${entity.yMilli})`;
    list.appendChild(item);
  }

  drawSnapshot(context, canvas, frame, summary.bounds);
})();

function drawSnapshot(context, canvas, frame, bounds) {
  const width = canvas.width;
  const height = canvas.height;
  const padding = 52;

  context.clearRect(0, 0, width, height);
  context.fillStyle = "#fbfaf8";
  context.fillRect(0, 0, width, height);

  context.strokeStyle = "#c7d2d7";
  context.lineWidth = 1;
  for (let x = padding; x <= width - padding; x += 80) {
    context.beginPath();
    context.moveTo(x, padding);
    context.lineTo(x, height - padding);
    context.stroke();
  }
  for (let y = padding; y <= height - padding; y += 80) {
    context.beginPath();
    context.moveTo(padding, y);
    context.lineTo(width - padding, y);
    context.stroke();
  }

  context.strokeStyle = "#334155";
  context.lineWidth = 2;
  context.strokeRect(padding, padding, width - padding * 2, height - padding * 2);

  const colors = ["#0f766e", "#b45309", "#2563eb", "#7c3aed"];
  frame.entities.forEach((entity, index) => {
    const point = project(entity, bounds, width, height, padding);
    context.fillStyle = colors[index % colors.length];
    context.beginPath();
    context.arc(point.x, point.y, 13, 0, Math.PI * 2);
    context.fill();

    context.fillStyle = "#111827";
    context.font = "16px Arial, Helvetica, sans-serif";
    context.fillText(entity.label, point.x + 18, point.y + 5);
  });

  context.fillStyle = "#334155";
  context.font = "14px Arial, Helvetica, sans-serif";
  context.fillText(`frame tick ${frame.atTicks}`, padding, 30);
}

function project(entity, bounds, width, height, padding) {
  const xRange = Math.max(1, bounds.maxXMilli - bounds.minXMilli);
  const yRange = Math.max(1, bounds.maxYMilli - bounds.minYMilli);
  return {
    x: padding + ((entity.xMilli - bounds.minXMilli) / xRange) * (width - padding * 2),
    y:
      height -
      padding -
      ((entity.yMilli - bounds.minYMilli) / yRange) * (height - padding * 2),
  };
}
