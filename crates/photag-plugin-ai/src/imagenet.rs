// ── ImageNet class index → user-friendly tag mapping ───────────────────
// Curated mapping of ImageNet 1000 classes → useful photo tags.
// Returns None for classes without meaningful tags.

pub(super) fn imagenet_to_tag(idx: usize) -> Option<&'static str> {
    Some(match idx {
        // ── Fish / aquatic ──────────────────────────────────────────
        0..=6 => "fish",
        // ── Birds ───────────────────────────────────────────────────
        7..=24 => "bird",
        // ── Amphibians / small creatures ────────────────────────────
        25..=32 => "frog",
        33..=68 => "sea creature",
        69..=70 => "crustacean",
        71..=76 => "spider",
        77..=99 => "insect",
        // ── Reptiles ────────────────────────────────────────────────
        100..=118 => "snake",
        119..=131 => "lizard",
        132..=142 => "dinosaur",
        143..=145 => "turtle",
        146..=150 => "lizard",
        // ── Dogs ────────────────────────────────────────────────────
        151..=268 => "dog",
        // ── Canids / wild ───────────────────────────────────────────
        269..=271 => "wolf",
        272..=274 => "fox",
        275 => "coyote",
        276..=280 => "animal",
        // ── Cats (domestic + big) ───────────────────────────────────
        281..=285 => "cat",
        286 => "cougar",
        287 => "lynx",
        288 => "leopard",
        289 => "snow leopard",
        290 => "jaguar",
        291 => "lion",
        292 => "tiger",
        293 => "cheetah",
        // ── Bears ───────────────────────────────────────────────────
        294..=297 => "bear",
        // ── Weasels / small mammals ─────────────────────────────────
        298..=299 => "animal",
        // ── Insects (continued) ─────────────────────────────────────
        300..=319 => "insect",
        320..=321 => "butterfly",
        322..=329 => "animal",
        // ── Rodents / rabbits ───────────────────────────────────────
        330..=332 => "rabbit",
        333..=338 => "animal",
        // ── Larger mammals ──────────────────────────────────────────
        339..=341 => "horse",
        342..=345 => "horse",
        346..=349 => "animal",
        350..=351 => "sheep",
        352..=353 => "cow",
        354..=355 => "pig",
        356..=364 => "animal",
        // ── Primates ────────────────────────────────────────────────
        365..=382 => "monkey",
        383..=384 => "animal",
        385..=386 => "elephant",
        387 => "animal",
        388 => "panda",
        389..=397 => "animal",
        // ── Music ───────────────────────────────────────────────────
        401..=402 => "music",
        404 => "airplane",
        407 => "car", // ambulance
        408 => "truck",
        436 => "car",   // beach wagon
        440 => "drink", // beer bottle
        444 => "bicycle",
        466 => "train",
        468 => "car",  // cab
        472 => "boat", // canoe
        481 => "camera",
        483 => "castle",
        484 => "boat",  // catamaran
        486 => "music", // cello
        487 => "phone", // cell phone
        497 => "church",
        508 => "computer",     // keyboard
        510 => "ship",         // container ship
        511 => "car",          // convertible
        513 => "music",        // cornet
        527 => "computer",     // desktop
        532 => "furniture",    // dining table
        536 => "architecture", // dome
        541..=542 => "music",  // drum
        546 => "music",        // electric guitar
        547 => "train",        // electric locomotive
        554 => "boat",         // fireboat
        555 => "truck",        // fire engine
        559 => "furniture",    // folding chair
        566 => "music",        // flute
        569 => "truck",        // fire truck
        574 => "sport",        // golf ball
        579 => "music",        // grand piano
        593 => "music",        // guitar
        609 => "car",          // jeep
        620 => "laptop",
        627 => "car",          // limousine
        628 => "boat",         // lifeboat
        642 => "music",        // marimba
        654 => "truck",        // moving van
        656 => "car",          // minivan
        663 => "architecture", // mosque
        664 => "monitor",
        668 => "architecture", // monastery
        671 => "motorcycle",
        681 => "computer",     // notebook
        687 => "music",        // organ
        698 => "architecture", // palace
        705 => "car",          // passenger car
        717 => "car",          // pickup truck
        722 => "sport",        // ping-pong ball
        724 => "boat",         // pirate ship
        734 => "car",          // police van
        737 => "sport",        // pool table
        747 => "sport",        // punching bag
        751 => "car",          // racer
        765 => "furniture",    // rocking chair
        768 => "sport",        // running shoe
        776 => "music",        // sax
        780 => "boat",         // schooner
        795 => "skiing",       // ski
        796 => "sport",        // ski mask
        801 => "soccer",       // soccer ball
        814 => "boat",         // speedboat
        817 => "car",          // sports car
        820 => "train",        // steam locomotive
        829 => "train",        // streetcar
        850 => "toy",          // teddy bear
        852 => "tennis",       // tennis ball
        864 => "truck",        // tow truck
        871 => "boat",         // trimaran
        875 => "music",        // trumpet
        889 => "music",        // violin
        895 => "airplane",     // warplane
        914 => "boat",         // yawl
        // ── Person-related (clothing, accessories, body parts) ──────
        // Massive block: any ImageNet class where a person is typically
        // visible in the image — clothing, headwear, eyewear, grooming.
        445 | 452 | 459 | 474 | 504 | 515 | 530 | 540 => "people",
        601 | 610 | 614 | 617 | 629 | 638 | 639 | 643 => "people",
        655 | 689 | 710 | 834 | 837 | 840 | 846 | 858 => "people",
        869 | 900 | 906 => "people",
        // Glasses / eyewear — critical for selfie detection
        831..=832 => "people", // studio couch (often with person)
        // Additional person-adjacent items in ImageNet:
        // 398: abaya, 399: academic gown, 403: altar boy (vestment)
        398..=400 => "people",
        // 414: apron, 425: backpack → typically worn by person
        414 => "people",
        425 => "people",
        // 431: baseball, 432: basketball → sport with people
        // 440: beer bottle → drink context, skip
        // 451: bib → baby
        451 => "people",
        // 456: bow tie
        456 => "people",
        // 469: cape, 479: cardigan (covered above 474)
        469 => "people",
        // 491: crash helmet, 497: church skip
        491 => "people",
        // 499: cloak
        499 => "people",
        // 509: confectionery → skip, 517: crutch → person
        517 => "people",
        // 518: cuirass (armor)
        518 => "people",
        // 529: dumbbell → person exercising
        529 => "people",
        // 531: ear → someone's ear visible (this is NOT glasses)
        // 548: embrace → hugging people
        // 552: face powder (cosmetics on face)
        552 => "people",
        // 589: fur coat
        589 => "people",
        // 608: jean / jeans → person wearing jeans
        608 => "people",
        // 618: lederhosen
        618 => "people",
        // 623: letter opener → skip, 626: lighter → skip
        // 650: military uniform
        650 => "people",
        // 696: pajama / pyjama
        696 => "people",
        // 725: pill bottle → skip, 753: racket → sport
        753 => "sport",
        // 770: running shoe → already in sport area
        // 783: scabbard (sword holder, worn)
        783 => "people",
        // 802: sombrero
        802 => "people",
        // 808: stethoscope → doctor / person
        808 => "people",
        // 824: stole (fur wrap)
        824 => "people",
        // 835: sunglasses (already in 846, but also here)
        // 842: sweatshirt
        842 => "people",
        // 856: swimming trunks → already in 840
        // 859: syringe → skip
        // 862: tank top / jersey
        // 866: toga
        866 => "people",
        // 888: veil
        888 => "people",
        // 910: wig → already in 900
        // ── Food ────────────────────────────────────────────────────
        924..=969 => "food",
        // ── Nature / landscape ──────────────────────────────────────
        970 => "mountain",
        971 => "nature",
        972 => "cliff",
        973 => "coast",
        974 => "nature", // geyser
        975 => "lake",
        976 => "beach",
        977 => "valley",
        978 => "volcano",
        979 => "mountain",
        980 => "nature",
        981..=984 => "nature",
        // ── Flowers / plants ────────────────────────────────────────
        985..=986 => "flower",
        987 => "plant",
        988 => "nature",
        989 => "mushroom",
        990..=999 => "nature",
        // Default: no useful tag
        _ => return None,
    })
}
