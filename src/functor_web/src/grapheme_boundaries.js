const segmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });

export function graphemeBoundaries(text) {
  const boundaries = [0];
  for (const part of segmenter.segment(text)) {
    const end = part.index + part.segment.length;
    if (end > boundaries[boundaries.length - 1]) {
      boundaries.push(end);
    }
  }
  return boundaries;
}

export function previousGraphemeBoundary(boundaries, offset) {
  let low = 0;
  let high = boundaries.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if (boundaries[middle] < offset) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  return low > 0 ? boundaries[low - 1] : null;
}

export function nextGraphemeBoundary(boundaries, offset) {
  let low = 0;
  let high = boundaries.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if (boundaries[middle] <= offset) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  return low < boundaries.length ? boundaries[low] : null;
}
