use std::rc::Rc;

struct Upgrades {
    upgrade_fingerprint: UpgradeFingerprint,
    item_type: Rc<str>,
    item_id: ItemID,
}

struct ItemID {
    oid: Rc<str>
}

struct UpgradeFingerprint {
    compat: Rc<str>,
    lim: i32,
    lvl_req: i32,
    lvl: i32,
    rerolls: i32,
    pol: Rc<str>,
    buffs: Vec<Buffs>,
    curses: Vec<Curses>,
}

struct Buffs {
    tag: Rc<str>,
    value: i32,
}

struct Curses {
    tag: Rc<str>,
    value: i32,
}

