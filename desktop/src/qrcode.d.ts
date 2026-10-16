declare module 'qrcode' {
  export function toCanvas(
    canvas: HTMLCanvasElement,
    text: string,
    options: { width: number; margin: number; errorCorrectionLevel: 'L' | 'M' | 'Q' | 'H' },
  ): Promise<HTMLCanvasElement>;
}
