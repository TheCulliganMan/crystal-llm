import { resolvePhoneContactId, tryAddPhoneNumber } from "./common";

describe("phone contact normalization", () => {
  it("canonicalizes Bike Shop non-trainer aliases to the PHONE_OAK slot from phone_contacts.asm", () => {
    expect(resolvePhoneContactId("PHONECONTACT_BIKESHOP")).toBe("PHONE_OAK");
    expect(resolvePhoneContactId("PHONE_BIKESHOP")).toBe("PHONE_OAK");
    expect(resolvePhoneContactId("PHONE_BIKE_SHOP")).toBe("PHONE_OAK");
  });

  it("deduplicates Bike Shop aliases against the canonical PHONE_OAK slot when adding numbers", () => {
    const phoneNumbers = ["PHONE_OAK"];

    expect(tryAddPhoneNumber(phoneNumbers, "PHONECONTACT_BIKESHOP")).toBe(false);
    expect(tryAddPhoneNumber(phoneNumbers, "PHONE_BIKESHOP")).toBe(false);
    expect(tryAddPhoneNumber(phoneNumbers, "PHONE_BIKE_SHOP")).toBe(false);
    expect(phoneNumbers).toEqual(["PHONE_OAK"]);
  });
});
