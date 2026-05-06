export class AnimationClock {
  public frame: number = 0;

  tick(): void {
    this.frame = (this.frame + 1) >>> 0;
  }
}
