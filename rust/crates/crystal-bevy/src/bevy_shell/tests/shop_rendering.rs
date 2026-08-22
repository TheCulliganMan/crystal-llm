use crate::core::systems::shop::ShopResult;

#[test]
fn mart_rendering_matches_the_typescript_text_contract() {
    let rendering = include_str!("../overworld_rendering.rs");
    let interactions = include_str!("../battle_messages.rs");
    let typescript = include_str!("../../../../../../packages/core/src/ui/menus/mart.ts");

    for text in [
        "Welcome! How may I\\nhelp you?",
        "Can I do anything\\nelse for you?",
        "Please come again!",
        "Here you are.\\nThank you!",
        "You don't have\\nenough money.",
        "You can't carry\\nany more items.",
        "You don't have anything to sell.",
        "You don't have any left.",
        "That item isn't for sale right now.",
        "BUY",
        "SELL",
        "QUIT",
    ] {
        assert!(
            rendering.contains(text) || interactions.contains(text),
            "mart UI is missing visible text {text:?}"
        );
    }

    assert!(rendering.contains("format_price(snapshot.trainer.money)"));
    assert!(rendering.contains("format_price(u32::from(item.price))"));
    assert!(rendering.contains("format!(\"×{:02} {}\", quantity.quantity, format_price(total))"));
    assert!(rendering.contains("visible_window_start(selected, item_ids.len(), 4)"));
    assert!(rendering.contains("battle_hud_tile_origin(1.0, 14.0 + index as f32 * 2.0)"));
    assert!(rendering.contains("let row = 4.0 + visible_index as f32 * 2.0;"));
    assert!(rendering.contains("battle_hud_tile_origin(2.0, row)"));
    assert!(rendering.contains("battle_hud_tile_origin(10.0, row + 1.0)"));
    assert!(rendering.contains("battle_hud_tile_origin(1.0, 13.0 + index as f32)"));
    assert!(rendering.contains("battle_hud_tile_origin(8.0, 16.0)"));
    for layout in [
        "topMenu: createWindow(0, 0, 8, 9)",
        "itemList: createWindow(1, 3, SCREEN_TILE_WIDTH - 1, MART_ITEM_LIST_HEIGHT_TILES)",
        "moneyWindow: createWindow(11, 0, SCREEN_TILE_WIDTH - 11, 3)",
        "quantityPrompt: createWindow(7, 15, SCREEN_TILE_WIDTH - 7, SCREEN_TILE_HEIGHT - 15)",
    ] {
        assert!(
            typescript.contains(layout),
            "TypeScript Mart layout changed: {layout}"
        );
    }
    assert!(!rendering.contains("let price = if selling { item.price / 2 } else { item.price };"));
    assert!(!rendering.contains("shop.mart_type, shop.mart_id, snapshot.trainer.money"));
    assert!(!rendering.contains("SELL {} ${}"));
    assert!(interactions.contains("Sorry, we're sold out."));
}

#[test]
fn mart_top_menu_text_stays_inside_its_window() {
    const LONGEST_OPTION_WIDTH: f32 = 5.0; // cursor/space plus "SELL" or "QUIT"
    const FORMATTED_MONEY_WIDTH: f32 = 7.0;

    assert!(SHOP_TOP_MENU_OPTION_LEFT >= SHOP_TOP_MENU_LEFT + 1.0);
    assert!(
        SHOP_TOP_MENU_OPTION_LEFT + LONGEST_OPTION_WIDTH
            <= SHOP_TOP_MENU_LEFT + SHOP_TOP_MENU_WIDTH - 1.0,
        "BUY/SELL/QUIT must render inside the top-menu frame"
    );
    assert!(SHOP_TOP_MENU_LEFT + SHOP_TOP_MENU_WIDTH <= SHOP_MONEY_WINDOW_LEFT);
    assert!(
        SHOP_MONEY_TEXT_LEFT + FORMATTED_MONEY_WIDTH
            <= SHOP_MONEY_WINDOW_LEFT + SHOP_MONEY_WINDOW_WIDTH - 1.0,
        "the six-digit money value must render inside the money frame"
    );
}

#[test]
fn mart_cancel_and_empty_buy_list_preserve_valid_menu_flow() {
    let session = include_str!("../deterministic_session.rs");
    let interactions = include_str!("../battle_messages.rs");

    let cancel_quantity = session
        .find("if runtime_shell.shop_quantity.take().is_some()")
        .expect("shop quantity cancellation branch");
    let next_shop_branch = session[cancel_quantity..]
        .find("if runtime_shell.shop_top_cursor.is_none()")
        .expect("top-menu fallback after quantity cancellation");
    assert!(
        session[cancel_quantity..cancel_quantity + next_shop_branch].contains("return Ok(());"),
        "cancelling a quantity prompt must return to its current item list"
    );
    assert!(interactions.contains("if shop.inventory.is_empty()"));
    assert!(interactions.contains("Sorry, we're sold out."));
}

#[test]
fn mart_cursor_and_sell_inventory_follow_the_reference_boundaries() {
    let navigation = include_str!("../credits.rs");
    let interactions = include_str!("../battle_messages.rs");

    assert!(navigation.contains("fn move_visible_mart_cursor_slot("));
    assert!(navigation.contains("current.saturating_sub(delta.unsigned_abs())"));
    assert!(navigation.contains(".min(option_count - 1)"));
    assert!(interactions.contains("snapshot\n            .bag\n            .tm_hm"));
}

#[test]
fn mart_item_cursors_clamp_at_both_ends() {
    let mut cursor = Some(MenuCursor {
        surface_id: "shop:test".to_string(),
        option_index: 0,
    });
    let mut events = Vec::new();

    move_visible_mart_cursor_slot(&mut cursor, "shop:test".to_string(), 4, -1, &mut events)
        .expect("clamp at first item");
    assert_eq!(cursor.as_ref().map(|cursor| cursor.option_index), Some(0));

    move_visible_mart_cursor_slot(&mut cursor, "shop:test".to_string(), 4, 99, &mut events)
        .expect("clamp at final item");
    assert_eq!(cursor.as_ref().map(|cursor| cursor.option_index), Some(3));

    move_visible_mart_cursor_slot(&mut cursor, "shop:test".to_string(), 4, -2, &mut events)
        .expect("move upward within list");
    assert_eq!(cursor.as_ref().map(|cursor| cursor.option_index), Some(1));
}

#[test]
fn mart_transaction_notices_use_player_facing_copy_and_prices() {
    let bought = ShopResult {
        success: true,
        message: format_price(300),
        credited: 300,
    };
    let sold = ShopResult {
        success: true,
        message: format_price(150),
        credited: 150,
    };
    let no_money = ShopResult {
        success: false,
        message: "You don't have enough money.".to_string(),
        credited: 0,
    };
    let pack_full = ShopResult {
        success: false,
        message: "Your Pack is full.".to_string(),
        credited: 0,
    };

    assert_eq!(format_price(0), "¥000000");
    assert_eq!(format_price(999_999), "¥999999");
    assert_eq!(
        visible_shop_transaction_status("BOUGHT", "POTION", &bought),
        "Here you are.\nThank you!"
    );
    assert_eq!(
        visible_shop_transaction_status("SOLD", "POTION", &sold),
        "Sold for ¥000150!"
    );
    assert_eq!(
        visible_shop_transaction_status("BOUGHT", "POTION", &no_money),
        "You don't have\nenough money."
    );
    assert_eq!(
        visible_shop_transaction_status("BOUGHT", "POTION", &pack_full),
        "You can't carry\nany more items."
    );
}
