export function intervalParts(seconds: number): { value: string; unit: number } {
  const unit = seconds % 3600 === 0 ? 3600 : seconds % 60 === 0 ? 60 : 1;
  return { value: String(seconds / unit), unit };
}

export function intervalSeconds(value: string, unit: number): number | null {
  const amount = Number(value);
  const seconds = amount * unit;
  return value.trim() && Number.isInteger(amount) && [1, 60, 3600].includes(unit)
    && seconds >= 1 && seconds <= 86400 ? seconds : null;
}
