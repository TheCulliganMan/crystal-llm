import { runPhoneTextScript } from "./helpers";
import type { ScriptRunner } from "../runner";

const createRunner = (callerId?: unknown): ScriptRunner =>
  ({
    variables: callerId === undefined ? {} : { VAR_CALLERID: callerId },
  }) as ScriptRunner;

describe("runPhoneTextScript", () => {
  it("throws instead of synthesizing the first male contact when VAR_CALLERID is missing", () => {
    expect(() => runPhoneTextScript(createRunner(), "AskNumber1Text")).toThrow(
      "Unknown phone contact '' for suffix 'AskNumber1Text'.",
    );
  });

  it("throws instead of rewriting unknown caller ids to a fallback contact", () => {
    expect(() => runPhoneTextScript(createRunner("PHONE_OAK"), "AskNumber1Text")).toThrow(
      "Unknown phone contact 'PHONE_OAK' for suffix 'AskNumber1Text'.",
    );
  });
});
