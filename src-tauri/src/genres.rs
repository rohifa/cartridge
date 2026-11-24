//! Cartridge's own genre vocabulary, filled from Steam user tags.
//!
//! Steam tags mix real genres with noise ("Great Soundtrack", "Singleplayer",
//! "Indie"). Instead of importing them, every genre below lists the Steam tag
//! IDs that mean it; a tag that isn't listed is ignored. Tag IDs come from
//! `IStoreService/GetTagList` and are stable (names may be renamed).
//!
//! A game gets at most `MAX_GENRES`, in the order Steam ranks the tags (most
//! votes first). The broad genres in `FALLBACK` are only used when none of the
//! specific ones matched, so a game is "Action" only when there is nothing
//! better to say about it.
//!
//! Genres are recomputed on every score run (the tags come with the review
//! request), so changes to the tables apply on the next launch.

/// How many of a game's top Steam tags are considered.
pub(crate) const TAG_COUNT: usize = 15;

/// Genres per game.
pub(crate) const MAX_GENRES: usize = 3;

/// Genre name → the Steam tags that mean it. Sentence case, as shown in the UI.
const SPECIFIC: &[(&str, &[u32])] = &[
    // Action
    ("Metroidvania", &[1628]),
    ("Soulslike", &[29482]),
    ("Roguelike", &[1716, 454187, 198631]), // Roguelike, Traditional Roguelike, Mystery Dungeon
    ("Roguelite", &[3959, 42804]),          // Roguelite, Action Roguelike
    ("Immersive sim", &[9204]),
    ("Stealth", &[1687]),
    ("Hack and slash", &[1646]),
    ("Character action", &[3955, 4777]), // Character Action Game, Spectacle fighter
    ("Beat 'em up", &[4158]),
    ("Musou", &[323922]),
    ("Fighting", &[1743, 4736, 6506]), // Fighting, 2D Fighter, 3D Fighter
    ("Bullet heaven", &[723991]),
    ("Boss rush", &[11095]),
    // Shooters
    ("First-person shooter", &[1663]),
    ("Boomer shooter", &[1023537, 5547]), // Boomer Shooter, Arena Shooter
    ("Third-person shooter", &[3814]),
    ("Hero shooter", &[620519]),
    ("Looter shooter", &[353880]),
    ("Extraction shooter", &[1199779]),
    ("Battle royale", &[176981]),
    ("Shoot 'em up", &[4255, 4885]), // Shoot 'Em Up, Bullet Hell
    ("Twin-stick shooter", &[4758, 4637]), // Twin Stick Shooter, Top-Down Shooter
    ("Rail shooter", &[3954, 56690]), // Rail Shooter, On-Rails Shooter
    // Platformers
    ("Platformer", &[1625, 5379, 5395]), // Platformer, 2D Platformer, 3D Platformer
    ("Precision platformer", &[3877]),
    ("Puzzle platformer", &[5537]),
    ("Collectathon", &[5652]),
    ("Endless runner", &[8666]),
    // Role-playing
    ("Action RPG", &[4231]),
    ("JRPG", &[4434]),
    ("CRPG", &[4474, 10695]),          // CRPG, Party-Based RPG
    ("Tactical RPG", &[21725, 17305]), // Tactical RPG, Strategy RPG
    ("Dungeon crawler", &[1720]),
    ("MMORPG", &[1754]),
    ("Creature collector", &[916648]),
    // Strategy
    ("Turn-based tactics", &[14139]),
    ("Real-time tactics", &[3813]),
    ("Real-time strategy", &[1676, 1723]), // RTS, Action RTS
    ("Grand strategy", &[4364]),
    ("4X", &[1670]),
    ("Wargame", &[4684]),
    ("Tower defense", &[1645]),
    ("Auto battler", &[1084988]),
    ("MOBA", &[1718]),
    ("Deckbuilder", &[32322, 1091588, 791774]), // Deckbuilding, Roguelike Deckbuilder, Card Battler
    ("Card game", &[1666, 9271, 13070]),        // Card Game, Trading Card Game, Solitaire
    ("Board game", &[1770, 17389, 4184, 33572]), // Board Game, Tabletop, Chess, Mahjong
    // Building and management
    ("City builder", &[4328]),
    ("Colony sim", &[220585]),
    ("Automation", &[255534]),
    ("Management", &[12472, 91114, 4845]), // Management, Shop Keeper, Capitalism
    ("God game", &[5300]),
    ("Survival", &[1662, 1100689]), // Survival, Open World Survival Craft
    ("Sandbox", &[3810]),
    ("Open world", &[1695]),
    // Life and cozy
    ("Farming sim", &[87918, 4520, 22602]), // Farming Sim, Farming, Agriculture
    ("Life sim", &[10235]),
    ("Dating sim", &[9551, 31579]), // Dating Sim, Otome
    ("Cozy", &[97376, 552282]),     // Cozy, Wholesome
    ("Cooking", &[3920]),
    // Simulation
    ("Flight sim", &[15045, 37799]), // Flight, Combat Flight Simulator
    ("Space sim", &[16598]),
    ("Vehicle sim", &[1100687, 1616, 10383]), // Automobile Sim, Trains, Transportation
    ("Job sim", &[35079]),
    ("Racing", &[699, 4102, 7622]), // Racing, Combat Racing, Offroad
    ("Sports", &[701, 1254552, 1254546, 1746, 7038, 5914, 1753]), // Sports, both Footballs, Basketball, Golf, Tennis, Skateboarding
    ("Fishing", &[15564]),
    // Horror
    ("Survival horror", &[3978]),
    ("Psychological horror", &[1721]),
    ("Horror", &[1667]),
    // Story
    ("Walking sim", &[5900]),
    ("Visual novel", &[3799]),
    ("Interactive fiction", &[11014, 31275, 4486]), // Interactive Fiction, Text-Based, Choose Your Own Adventure
    ("Point and click", &[1698]),
    ("Detective", &[5613, 8369]), // Detective, Investigation
    ("Hidden object", &[1738]),
    // Puzzle and other
    ("Puzzle", &[1664, 6129, 1730, 1665, 37376]), // Puzzle, Logic, Sokoban, Match 3, Falling Blocks
    ("Escape room", &[769306]),
    ("Programming", &[5432, 42329]),      // Programming, Coding
    ("Word game", &[24003, 10437, 1674]), // Word Game, Trivia, Typing
    ("Rhythm", &[1752]),
    ("Party", &[7178, 7108, 8093]), // Party Game, Party, Minigames
    ("Social deduction", &[745697]),
    ("Incremental", &[615955, 560542, 379975]), // Idler, Incremental, Clicker
    ("Pinball", &[6621]),
];

/// Broad genres, used only when nothing in `SPECIFIC` matched.
const FALLBACK: &[(&str, &[u32])] = &[
    ("Action-adventure", &[4106]),
    ("Action", &[19]),
    ("Adventure", &[21]),
    ("Role-playing", &[122]),
    ("Strategy", &[9]),
    ("Simulation", &[599]),
    ("Shooter", &[1774]),
    ("Arcade", &[1773]),
];

fn lookup(table: &[(&'static str, &[u32])], tag: u32) -> Option<&'static str> {
    table
        .iter()
        .find(|(_, tags)| tags.contains(&tag))
        .map(|(name, _)| *name)
}

/// A genre that says nothing more once one of its narrower genres is there:
/// "Precision platformer" makes "Platformer" redundant.
const NARROWER: &[(&str, &[&str])] = &[
    ("Platformer", &["Precision platformer", "Puzzle platformer"]),
    ("Horror", &["Survival horror", "Psychological horror"]),
];

fn redundant(name: &str, found: &[&str]) -> bool {
    NARROWER
        .iter()
        .any(|(broad, narrow)| *broad == name && narrow.iter().any(|n| found.contains(n)))
}

fn collect(table: &[(&'static str, &[u32])], tags: &[u32]) -> Vec<String> {
    let mut found: Vec<&str> = Vec::new();
    for &tag in tags.iter().take(TAG_COUNT) {
        if let Some(name) = lookup(table, tag) {
            if !found.contains(&name) {
                found.push(name);
            }
        }
    }
    found
        .iter()
        .filter(|name| !redundant(name, &found))
        .take(MAX_GENRES)
        .map(|name| name.to_string())
        .collect()
}

/// Genres for a game from its Steam tag IDs, most voted first. Empty when no
/// tag maps to a genre.
pub(crate) fn from_steam_tags(tags: &[u32]) -> Vec<String> {
    let specific = collect(SPECIFIC, tags);
    if specific.is_empty() {
        collect(FALLBACK, tags)
    } else {
        specific
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn keeps_specific_genres_in_steam_order() {
        // Hollow Knight: Metroidvania, Platformer, Souls-like, Difficult,
        // Great Soundtrack, 2D, Indie, Singleplayer.
        let tags = [1628, 1625, 29482, 4026, 1756, 3871, 492, 4182];
        assert_eq!(
            from_steam_tags(&tags),
            ["Metroidvania", "Platformer", "Soulslike"]
        );
    }

    #[test]
    fn merges_tags_that_mean_the_same_genre() {
        // Action Roguelike, Roguelite, Indie, Hack and Slash
        assert_eq!(
            from_steam_tags(&[42804, 3959, 492, 1646]),
            ["Roguelite", "Hack and slash"]
        );
    }

    #[test]
    fn broad_genres_only_when_nothing_specific_matched() {
        // Action, Adventure, Indie, Singleplayer
        assert_eq!(
            from_steam_tags(&[19, 21, 492, 4182]),
            ["Action", "Adventure"]
        );
        // Action, Metroidvania: the specific one wins, Action is dropped.
        assert_eq!(from_steam_tags(&[19, 1628]), ["Metroidvania"]);
    }

    #[test]
    fn ignores_noise_and_low_ranked_tags() {
        // Great Soundtrack, Singleplayer, Indie, Casual
        assert!(from_steam_tags(&[1756, 4182, 492, 597]).is_empty());
        let mut tags = vec![1756; TAG_COUNT];
        tags.push(1628);
        assert!(from_steam_tags(&tags).is_empty());
    }

    #[test]
    fn drops_a_broad_genre_when_a_narrower_one_is_there() {
        // Celeste: Precision Platformer, Platformer, Puzzle Platformer, Pixel Graphics
        assert_eq!(
            from_steam_tags(&[3877, 1625, 5537, 3964]),
            ["Precision platformer", "Puzzle platformer"]
        );
        // Horror, Survival Horror, Zombies, Puzzle
        assert_eq!(
            from_steam_tags(&[1667, 3978, 1659, 1664]),
            ["Survival horror", "Puzzle"]
        );
        assert_eq!(from_steam_tags(&[1667, 1664]), ["Horror", "Puzzle"]);
    }

    #[test]
    fn caps_the_number_of_genres() {
        let tags = [1628, 29482, 1716, 1687, 1646];
        assert_eq!(from_steam_tags(&tags).len(), MAX_GENRES);
    }

    #[test]
    fn every_tag_and_name_is_used_once() {
        let mut tags = HashSet::new();
        let mut names = HashSet::new();
        for (name, ids) in SPECIFIC.iter().chain(FALLBACK) {
            assert!(names.insert(*name), "genre {name} listed twice");
            for id in *ids {
                assert!(tags.insert(*id), "tag {id} mapped twice");
            }
        }
    }
}
