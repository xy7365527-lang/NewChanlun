/** Bitmap positioning helpers (from LW Charts plugin-examples) */

export interface BitmapPositionLength {
  position: number;
  length: number;
}

export function positionsBox(
  position1Media: number,
  position2Media: number,
  pixelRatio: number
): BitmapPositionLength {
  const scaledPosition1 = Math.round(pixelRatio * position1Media);
  const scaledPosition2 = Math.round(pixelRatio * position2Media);
  return {
    position: Math.min(scaledPosition1, scaledPosition2),
    length: Math.abs(scaledPosition2 - scaledPosition1) + 1,
  };
}

export function positionsLine(
  positionMedia: number,
  pixelRatio: number,
  desiredWidthMedia: number = 1
): BitmapPositionLength {
  const scaledPosition = Math.round(pixelRatio * positionMedia);
  const lineBitmapWidth = Math.round(desiredWidthMedia * pixelRatio);
  const offset = Math.floor(lineBitmapWidth * 0.5);
  return { position: scaledPosition - offset, length: lineBitmapWidth };
}
