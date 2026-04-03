import type { NodeHoverDrawingFunction, NodeLabelDrawingFunction } from 'sigma/rendering';

const ARABIC_RE = /[\u0600-\u06FF]/;

function isRTL(text: string): boolean {
  return ARABIC_RE.test(text.charAt(0));
}

function getIsLight(): boolean {
  if (typeof document === 'undefined') return true;
  const theme = document.documentElement.getAttribute('data-theme');
  return !theme || theme !== 'dark';
}

export const drawLabel: NodeLabelDrawingFunction = (context, data, settings) => {
  if (!data.label) return;

  const size = data.labelSize || settings.labelSize;
  const font = settings.labelFont;
  const weight = settings.labelWeight;
  const color = data.labelColor || settings.labelColor.color;

  context.fillStyle = color;
  context.font = `${weight} ${size}px ${font}`;

  if (isRTL(data.label)) {
    context.direction = 'rtl';
    context.textAlign = 'right';
    context.fillText(data.label, data.x - data.size - 3, data.y + size / 3);
  } else {
    context.direction = 'ltr';
    context.textAlign = 'left';
    context.fillText(data.label, data.x + data.size + 3, data.y + size / 3);
  }
};

export const drawHover: NodeHoverDrawingFunction = (context, data, settings) => {
  const size = data.labelSize || settings.labelSize;
  const font = settings.labelFont;
  const weight = settings.labelWeight;
  const light = getIsLight();

  context.font = `${weight} ${size}px ${font}`;
  context.fillStyle = light ? '#FFF' : '#000';

  const PADDING = 4;

  if (typeof data.label === 'string' && data.label) {
    const textWidth = context.measureText(data.label).width;
    const boxWidth = Math.round(textWidth + 5);
    const boxHeight = Math.round(size + 2 * PADDING);
    const radius = Math.max(data.size, size / 2) + PADDING;

    const angleRadian = Math.asin(boxHeight / 2 / radius);
    const xDeltaCoord = Math.sqrt(Math.abs(radius ** 2 - (boxHeight / 2) ** 2));

    if (isRTL(data.label)) {
      // Draw pill to the left for Arabic
      context.beginPath();
      context.moveTo(data.x - xDeltaCoord, data.y + boxHeight / 2);
      context.lineTo(data.x - radius - boxWidth, data.y + boxHeight / 2);
      context.lineTo(data.x - radius - boxWidth, data.y - boxHeight / 2);
      context.lineTo(data.x - xDeltaCoord, data.y - boxHeight / 2);
      context.arc(data.x, data.y, radius, Math.PI - angleRadian, Math.PI + angleRadian, true);
      context.closePath();
      context.fill();
    } else {
      // Draw pill to the right (Surrealist default)
      context.beginPath();
      context.moveTo(data.x + xDeltaCoord, data.y + boxHeight / 2);
      context.lineTo(data.x + radius + boxWidth, data.y + boxHeight / 2);
      context.lineTo(data.x + radius + boxWidth, data.y - boxHeight / 2);
      context.lineTo(data.x + xDeltaCoord, data.y - boxHeight / 2);
      context.arc(data.x, data.y, radius, angleRadian, -angleRadian);
      context.closePath();
      context.fill();
    }
  } else {
    context.beginPath();
    context.arc(data.x, data.y, data.size + PADDING, 0, Math.PI * 2);
    context.closePath();
    context.fill();
  }

  drawLabel(context, data, settings);
};
