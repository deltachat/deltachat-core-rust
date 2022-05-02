/**
 * @param integerColor expects a 24bit rgb integer (left to right: 8bits red, 8bits green, 8bits blue)
 */
export function integerToHexColor(integerColor: number) {
  return '#' + (integerColor + 16777216).toString(16).substring(1)
}
