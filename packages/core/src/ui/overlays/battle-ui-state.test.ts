import { compute_hp_pixels } from "./battle-bars";
import { HPBarAnimationState } from "./battle-ui-state";

describe("HPBarAnimationState", () => {
  it("animates upward when HP increases", () => {
    const animation = new HPBarAnimationState();
    const maxHp = 100;
    const startPixels = compute_hp_pixels(10, maxHp);
    const targetPixels = compute_hp_pixels(30, maxHp);

    animation.sync(startPixels, maxHp);
    animation.sync(targetPixels, maxHp);

    const initialPixels = animation.current_pixels;
    animation.step();

    expect(animation.current_pixels).toBeGreaterThan(initialPixels);

    for (let i = 0; i < 200; i += 1) {
      animation.step();
      if (!animation.active) {
        break;
      }
    }

    expect(animation.current_pixels).toBe(targetPixels);
  });

  it("snaps to the new bar length when the battler changes even if max HP matches", () => {
    const animation = new HPBarAnimationState();
    const maxHp = 100;
    const outgoing = { id: "outgoing" };
    const incoming = { id: "incoming" };
    const outgoingPixels = compute_hp_pixels(10, maxHp);
    const incomingPixels = compute_hp_pixels(30, maxHp);

    animation.sync(outgoingPixels, maxHp, { subject_token: outgoing });
    animation.sync(incomingPixels, maxHp, { subject_token: incoming });

    expect(animation.current_pixels).toBe(incomingPixels);
    expect(animation.target_pixels).toBe(incomingPixels);
    expect(animation.active).toBe(false);
  });
});
