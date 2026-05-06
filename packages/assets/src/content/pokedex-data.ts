import { z } from 'zod';

export const pokedexDataSchema = z.object({
  species: z.string(),
  classification: z.string(),
  height: z.number(),
  weight: z.number(),
  text: z.string(),
});

export type PokedexData = z.infer<typeof pokedexDataSchema>;

export const pokedexData: PokedexData[] = [
  {
    "species": "ABRA",
    "classification": "PSI",
    "height": 0.89,
    "weight": 19.5,
    "text": "It hypnotizes itself so that it can teleport away when it senses danger, even if it is asleep."
  },
  {
    "species": "AERODACTYL",
    "classification": "FOSSIL",
    "height": 1.8,
    "weight": 58.97,
    "text": "In prehistoric times, this #MON flew freely and fearlessly through the skies."
  },
  {
    "species": "AIPOM",
    "classification": "LONG TAIL",
    "height": 0.79,
    "weight": 11.34,
    "text": "It uses its tail to hang on to tree branches. It uses its momentum to swing from one branch to another."
  },
  {
    "species": "ALAKAZAM",
    "classification": "PSI",
    "height": 1.5,
    "weight": 48.08,
    "text": "It has an IQ of 5000. It calculates many things in order to gain the edge in every battle."
  },
  {
    "species": "AMPHAROS",
    "classification": "LIGHT",
    "height": 1.4,
    "weight": 61.69,
    "text": "When it gets dark, the light from its bright, shiny tail can be seen from far away on the ocean's surface."
  },
  {
    "species": "ARBOK",
    "classification": "COBRA",
    "height": 3.51,
    "weight": 64.86,
    "text": "To intimidate foes, it spreads its chest wide and makes eerie sounds by expelling air from its mouth."
  },
  {
    "species": "ARCANINE",
    "classification": "LEGENDARY",
    "height": 1.91,
    "weight": 155.13,
    "text": "An ancient picture scroll shows that people were attracted to its movement as it ran through prairies."
  },
  {
    "species": "ARIADOS",
    "classification": "LONG LEG",
    "height": 1.09,
    "weight": 33.57,
    "text": "Rather than making a nest in one specific spot, it wanders in search of food after darkness falls."
  },
  {
    "species": "ARTICUNO",
    "classification": "FREEZE",
    "height": 1.7,
    "weight": 55.34,
    "text": "Legendary bird #MON. As it flies through the sky, it cools the air, causing snow to fall."
  },
  {
    "species": "AZUMARILL",
    "classification": "AQUARABBIT",
    "height": 0.79,
    "weight": 28.58,
    "text": "The bubble-like pattern on its stomach helps it camouflage itself when it's in the water."
  },
  {
    "species": "BAYLEEF",
    "classification": "LEAF",
    "height": 1.19,
    "weight": 15.88,
    "text": "The scent that wafts from the leaves on its neck causes anyone who smells it to become energetic."
  },
  {
    "species": "BEEDRILL",
    "classification": "POISON BEE",
    "height": 0.99,
    "weight": 29.48,
    "text": "It uses sharp, poisonous stings to defeat prey, then takes the victim back to its nest for food."
  },
  {
    "species": "BELLOSSOM",
    "classification": "FLOWER",
    "height": 0.41,
    "weight": 5.9,
    "text": "When these dance together, their petals rub against each other, making pretty, relaxing sounds."
  },
  {
    "species": "BELLSPROUT",
    "classification": "FLOWER",
    "height": 0.71,
    "weight": 4.08,
    "text": "If it notices anything that moves, it immediately flings its vine at the object."
  },
  {
    "species": "BLASTOISE",
    "classification": "SHELLFISH",
    "height": 1.6,
    "weight": 85.73,
    "text": "It firmly plants its feet on the ground before shooting water from the jets on its back."
  },
  {
    "species": "BLISSEY",
    "classification": "HAPPINESS",
    "height": 1.5,
    "weight": 46.72,
    "text": "Biting into one of the delicious eggs that BLISSEY provides will make everyone around smile with joy."
  },
  {
    "species": "BULBASAUR",
    "classification": "SEED",
    "height": 0.71,
    "weight": 6.8,
    "text": "While it is young, it uses the nutrients that are stored in the seeds on its back in order to grow."
  },
  {
    "species": "BUTTERFREE",
    "classification": "BUTTERFLY",
    "height": 1.09,
    "weight": 32.21,
    "text": "It flits from flower to flower, collecting honey. It can even identify distant flowers in bloom."
  },
  {
    "species": "CATERPIE",
    "classification": "WORM",
    "height": 0.3,
    "weight": 2.72,
    "text": "It crawls into foliage where it camouflages itself among leaves that are the same color as its body."
  },
  {
    "species": "CELEBI",
    "classification": "TIMETRAVEL",
    "height": 0.61,
    "weight": 4.99,
    "text": "Revered as a guardian of the forest, CELEBI appears wherever beautiful forests exist."
  },
  {
    "species": "CHANSEY",
    "classification": "EGG",
    "height": 1.09,
    "weight": 34.47,
    "text": "People try to catch it for its extremely nutritious eggs, but it rarely can be found."
  },
  {
    "species": "CHARIZARD",
    "classification": "FLAME",
    "height": 1.7,
    "weight": 90.72,
    "text": "It uses its wings to fly high. The temperature of its fire increases as it gains experience in battle."
  },
  {
    "species": "CHARMANDER",
    "classification": "LIZARD",
    "height": 0.61,
    "weight": 8.62,
    "text": "If it's healthy, the flame on the tip of its tail will burn vigorously, even if it gets a bit wet."
  },
  {
    "species": "CHARMELEON",
    "classification": "FLAME",
    "height": 1.09,
    "weight": 19.05,
    "text": "If it becomes agitated during battle, it spouts intense flames, incinerating its surroundings."
  },
  {
    "species": "CHIKORITA",
    "classification": "LEAF",
    "height": 0.89,
    "weight": 6.35,
    "text": "It loves to bask in the sunlight. It uses the leaf on its head to seek out warm places."
  },
  {
    "species": "CHINCHOU",
    "classification": "ANGLER",
    "height": 0.51,
    "weight": 11.79,
    "text": "Its antennae, which evolved from a fin, have both positive and negative charges flowing through them."
  },
  {
    "species": "CLEFABLE",
    "classification": "FAIRY",
    "height": 1.3,
    "weight": 39.92,
    "text": "Said to live in quiet, remote mountains, this type of fairy has a strong aversion to being seen."
  },
  {
    "species": "CLEFAIRY",
    "classification": "FAIRY",
    "height": 0.61,
    "weight": 7.71,
    "text": "Though rarely seen, it becomes easier to spot, for some reason, on the night of a full moon."
  },
  {
    "species": "CLEFFA",
    "classification": "STARSHAPE",
    "height": 0.3,
    "weight": 3.18,
    "text": "If the impact site of a meteorite is found, this #MON is certain to be within the immediate area."
  },
  {
    "species": "CLOYSTER",
    "classification": "BIVALVE",
    "height": 1.5,
    "weight": 132.45,
    "text": "Even a missile can't break the spikes it uses to stab opponents. They're even harder than its shell."
  },
  {
    "species": "CORSOLA",
    "classification": "CORAL",
    "height": 0.61,
    "weight": 4.99,
    "text": "The points on its head absorb nutrients from clean water. They cannot survive in polluted water."
  },
  {
    "species": "CROBAT",
    "classification": "BAT",
    "height": 1.8,
    "weight": 74.84,
    "text": "As a result of its pursuit of faster, yet more silent flight, a new set of wings grew on its hind legs."
  },
  {
    "species": "CROCONAW",
    "classification": "BIG JAW",
    "height": 1.09,
    "weight": 24.95,
    "text": "The tips of its fangs are slanted backward. Once those fangs clamp down, the prey has no hope of escape."
  },
  {
    "species": "CUBONE",
    "classification": "LONELY",
    "height": 0.41,
    "weight": 6.35,
    "text": "It lost its mother after its birth. It wears its mother's skull, never revealing its true face."
  },
  {
    "species": "CYNDAQUIL",
    "classification": "FIRE MOUSE",
    "height": 0.51,
    "weight": 7.71,
    "text": "The fire that spouts from its back burns hottest when it is angry. The flaring flames intimidate foes."
  },
  {
    "species": "DELIBIRD",
    "classification": "DELIVERY",
    "height": 0.89,
    "weight": 15.88,
    "text": "It always carries its food with it, wherever it goes. If attacked, it throws its food at the opponent."
  },
  {
    "species": "DEWGONG",
    "classification": "SEA LION",
    "height": 1.7,
    "weight": 120.2,
    "text": "It sleeps under shallow ocean waters during the day, then looks for food at night when it's cold."
  },
  {
    "species": "DIGLETT",
    "classification": "MOLE",
    "height": 0.2,
    "weight": 0.91,
    "text": "It digs underground and chews on tree roots, sticking its head out only when the sun isn't bright."
  },
  {
    "species": "DITTO",
    "classification": "TRANSFORM",
    "height": 0.3,
    "weight": 4.08,
    "text": "When it encounters another DITTO, it will move faster than normal to duplicate that opponent exactly."
  },
  {
    "species": "DODRIO",
    "classification": "TRIPLEBIRD",
    "height": 1.8,
    "weight": 85.28,
    "text": "An enemy that takes its eyes off any of the three heads--even for a second--will get pecked severely."
  },
  {
    "species": "DODUO",
    "classification": "TWIN BIRD",
    "height": 1.4,
    "weight": 39.01,
    "text": "It lives on a grassy plain where it can see a long way. If it sees an enemy, it runs away at 60 mph."
  },
  {
    "species": "DONPHAN",
    "classification": "ARMOR",
    "height": 1.09,
    "weight": 120.2,
    "text": "Because this #MON's skin is so tough, a normal attack won't even leave a scratch on it."
  },
  {
    "species": "DRAGONAIR",
    "classification": "DRAGON",
    "height": 3.99,
    "weight": 16.33,
    "text": "It is called the divine #MON. When its entire body brightens slightly, the weather changes."
  },
  {
    "species": "DRAGONITE",
    "classification": "DRAGON",
    "height": 2.21,
    "weight": 210.01,
    "text": "It is said that somewhere in the ocean lies an island where these gather. Only they live there."
  },
  {
    "species": "DRATINI",
    "classification": "DRAGON",
    "height": 1.8,
    "weight": 3.18,
    "text": "It sheds many layers of skin as it grows larger. During this process, it is protected by a rapid waterfall."
  },
  {
    "species": "DROWZEE",
    "classification": "HYPNOSIS",
    "height": 0.99,
    "weight": 32.21,
    "text": "When it twitches its nose, it can tell where someone is sleeping and what that person is dreaming about."
  },
  {
    "species": "DUGTRIO",
    "classification": "MOLE",
    "height": 0.71,
    "weight": 33.11,
    "text": "These DIGLETT triplets dig over 60 miles below sea level. No one knows what it's like underground."
  },
  {
    "species": "DUNSPARCE",
    "classification": "LAND SNAKE",
    "height": 1.5,
    "weight": 14.06,
    "text": "It hides deep inside caves where no light ever reaches it and remains virtually motionless there."
  },
  {
    "species": "EEVEE",
    "classification": "EVOLUTION",
    "height": 0.3,
    "weight": 6.35,
    "text": "Its ability to evolve into many forms allows it to adapt smoothly and perfectly to any environment."
  },
  {
    "species": "EKANS",
    "classification": "SNAKE",
    "height": 2.01,
    "weight": 6.8,
    "text": "It flutters the tip of its tongue to seek out the scent of prey, then swallows the prey whole."
  },
  {
    "species": "ELECTABUZZ",
    "classification": "ELECTRIC",
    "height": 1.09,
    "weight": 29.94,
    "text": "When two ELECTABUZZ touch, they control the electric currents to communicate their feelings."
  },
  {
    "species": "ELECTRODE",
    "classification": "BALL",
    "height": 1.19,
    "weight": 66.68,
    "text": "The more energy it charges up, the faster it gets. But this also makes it more likely to explode."
  },
  {
    "species": "ELEKID",
    "classification": "ELECTRIC",
    "height": 0.61,
    "weight": 23.59,
    "text": "It loves violent thunder. The space between its horns flickers bluishwhite when it is charging energy."
  },
  {
    "species": "ENTEI",
    "classification": "VOLCANO",
    "height": 2.01,
    "weight": 198.22,
    "text": "This brawny #MON courses around the earth, spouting flames hotter than a volcano's magma."
  },
  {
    "species": "ESPEON",
    "classification": "SUN",
    "height": 0.89,
    "weight": 26.31,
    "text": "The tip of its forked tail quivers when it is predicting its opponent's next move."
  },
  {
    "species": "EXEGGCUTE",
    "classification": "EGG",
    "height": 0.41,
    "weight": 2.72,
    "text": "If even one is separated from the group, the energy bond between the six will make them rejoin instantly."
  },
  {
    "species": "EXEGGUTOR",
    "classification": "COCONUT",
    "height": 2.01,
    "weight": 120.2,
    "text": "Living in a good environment makes it grow lots of heads. A head that drops off becomes an EXEGGCUTE."
  },
  {
    "species": "FARFETCH_D",
    "classification": "WILD DUCK",
    "height": 0.79,
    "weight": 14.97,
    "text": "In order to prevent their extinction, more people have made an effort to breed these #MON."
  },
  {
    "species": "FEAROW",
    "classification": "BEAK",
    "height": 1.19,
    "weight": 38.1,
    "text": "It uses its long beak to attack. It has a surprisingly long reach, so it must be treated with caution."
  },
  {
    "species": "FERALIGATR",
    "classification": "BIG JAW",
    "height": 2.31,
    "weight": 88.9,
    "text": "Although it has a massive body, its powerful hind legs enable it to move quickly, even on the ground."
  },
  {
    "species": "FLAAFFY",
    "classification": "WOOL",
    "height": 0.79,
    "weight": 13.15,
    "text": "Because of its rubbery, electricity-resistant skin, it can store lots of electricity in its fur."
  },
  {
    "species": "FLAREON",
    "classification": "FLAME",
    "height": 0.89,
    "weight": 24.95,
    "text": "Once it has stored up enough heat, this #MON's body temperature can reach up to 1700 degrees."
  },
  {
    "species": "FORRETRESS",
    "classification": "BAGWORM",
    "height": 1.19,
    "weight": 125.65,
    "text": "Usually found hanging on to a fat tree trunk. It shoots out bits of its shell when it sees action."
  },
  {
    "species": "FURRET",
    "classification": "LONG BODY",
    "height": 1.8,
    "weight": 32.66,
    "text": "It lives in narrow burrows that fit its slim body. The deeper the nests go, the more mazelike they become."
  },
  {
    "species": "GASTLY",
    "classification": "GAS",
    "height": 1.3,
    "weight": 0.09,
    "text": "It wraps its opponent in its gaslike body, slowly weakening its prey by poisoning it through the skin."
  },
  {
    "species": "GENGAR",
    "classification": "SHADOW",
    "height": 1.5,
    "weight": 40.37,
    "text": "Hiding in people's shadows at night, it absorbs their heat. The chill it causes makes the victims shake."
  },
  {
    "species": "GEODUDE",
    "classification": "ROCK",
    "height": 0.41,
    "weight": 19.96,
    "text": "Proud of their sturdy bodies, they bash against each other in a contest to prove whose is harder."
  },
  {
    "species": "GIRAFARIG",
    "classification": "LONG NECK",
    "height": 1.5,
    "weight": 41.28,
    "text": "When it is in danger, its tail uses some sort of mysterious powers to drive away the enemy."
  },
  {
    "species": "GLIGAR",
    "classification": "FLYSCORPIO",
    "height": 1.09,
    "weight": 64.86,
    "text": "It builds its nest on a steep cliff. When it is done gliding, it hops along the ground back to its nest."
  },
  {
    "species": "GLOOM",
    "classification": "WEED",
    "height": 0.79,
    "weight": 8.62,
    "text": "The smell from its drool-like syrup and the pollen on its petals is so bad, it may make opponents faint."
  },
  {
    "species": "GOLBAT",
    "classification": "BAT",
    "height": 1.6,
    "weight": 54.88,
    "text": "When it plunges its fangs into its prey, it instantly draws and gulps down more than ten ounces of blood."
  },
  {
    "species": "GOLDEEN",
    "classification": "GOLDFISH",
    "height": 0.61,
    "weight": 14.97,
    "text": "During spawning season, they swim gracefully in the water, searching for their perfect mate."
  },
  {
    "species": "GOLDUCK",
    "classification": "DUCK",
    "height": 1.7,
    "weight": 76.66,
    "text": "It swims gracefully along on the quiet, slow-moving rivers and lakes of which it is so fond."
  },
  {
    "species": "GOLEM",
    "classification": "MEGATON",
    "height": 1.4,
    "weight": 300.28,
    "text": "Its rock-like body is so durable, even high-powered dynamite blasts fail to scratch its rugged hide."
  },
  {
    "species": "GRANBULL",
    "classification": "FAIRY",
    "height": 1.4,
    "weight": 48.53,
    "text": "It can make most any #MON run away simply by opening its mouth wide to reveal its big fangs."
  },
  {
    "species": "GRAVELER",
    "classification": "ROCK",
    "height": 0.99,
    "weight": 105.23,
    "text": "It travels by rolling on mountain paths. If it gains too much speed, it stops by running into huge rocks."
  },
  {
    "species": "GRIMER",
    "classification": "SLUDGE",
    "height": 0.89,
    "weight": 29.94,
    "text": "When two of these #MON's bodies are combined together, new poisons are created."
  },
  {
    "species": "GROWLITHE",
    "classification": "PUPPY",
    "height": 0.71,
    "weight": 19.05,
    "text": "It controls a big territory. If it detects an unknown smell, it roars loudly to force out the intruder."
  },
  {
    "species": "GYARADOS",
    "classification": "ATROCIOUS",
    "height": 6.5,
    "weight": 234.96,
    "text": "It appears whenever there is world conflict, burning down any place it travels through."
  },
  {
    "species": "HAUNTER",
    "classification": "GAS",
    "height": 1.6,
    "weight": 0.09,
    "text": "It hides in the dark, planning to take the life of the next living thing that wanders close by."
  },
  {
    "species": "HERACROSS",
    "classification": "SINGLEHORN",
    "height": 1.5,
    "weight": 53.98,
    "text": "With its Herculean powers, it can easily throw around an object that is 100 times its own weight."
  },
  {
    "species": "HITMONCHAN",
    "classification": "PUNCHING",
    "height": 1.4,
    "weight": 50.35,
    "text": "To increase the strength of all its punch moves, it spins its arms just before making contact."
  },
  {
    "species": "HITMONLEE",
    "classification": "KICKING",
    "height": 1.5,
    "weight": 49.9,
    "text": "It is also called the Kick Master. It uses its elastic legs to execute every known kick."
  },
  {
    "species": "HITMONTOP",
    "classification": "HANDSTAND",
    "height": 1.4,
    "weight": 48.08,
    "text": "After doing a handstand to throw off the opponent's timing, it presents its fancy kick moves."
  },
  {
    "species": "HO_OH",
    "classification": "RAINBOW",
    "height": 3.81,
    "weight": 199.13,
    "text": "It will reveal itself before a pure-hearted trainer by shining its bright rainbow-colored wings."
  },
  {
    "species": "HOOTHOOT",
    "classification": "OWL",
    "height": 0.71,
    "weight": 21.32,
    "text": "It begins to hoot at the same time every day. Some trainers use them in place of clocks."
  },
  {
    "species": "HOPPIP",
    "classification": "COTTONWEED",
    "height": 0.41,
    "weight": 0.45,
    "text": "It can be carried away on even the gentlest breeze. It may even float all the way to the next town."
  },
  {
    "species": "HORSEA",
    "classification": "DRAGON",
    "height": 0.41,
    "weight": 8.16,
    "text": "When they're in a safe location, they can be seen playfully tangling their tails together."
  },
  {
    "species": "HOUNDOOM",
    "classification": "DARK",
    "height": 1.4,
    "weight": 34.93,
    "text": "The pungentsmelling flame that shoots from its mouth results from toxins burning in its body."
  },
  {
    "species": "HOUNDOUR",
    "classification": "DARK",
    "height": 0.61,
    "weight": 10.89,
    "text": "Around dawn, its ominous howl echoes through the area to announce that this is its territory."
  },
  {
    "species": "HYPNO",
    "classification": "HYPNOSIS",
    "height": 1.6,
    "weight": 75.75,
    "text": "The longer it swings its pendulum, the longer the effects of its hypnosis last."
  },
  {
    "species": "IGGLYBUFF",
    "classification": "BALLOON",
    "height": 0.3,
    "weight": 0.91,
    "text": "Instead of walking with its short legs, it moves around by bouncing on its soft, tender body."
  },
  {
    "species": "IVYSAUR",
    "classification": "SEED",
    "height": 0.99,
    "weight": 13.15,
    "text": "The bulb on its back grows as it absorbs nutrients. The bulb gives off a pleasant aroma when it blooms."
  },
  {
    "species": "JIGGLYPUFF",
    "classification": "BALLOON",
    "height": 0.51,
    "weight": 5.44,
    "text": "It rolls its cute eyes as it sings a soothing lullaby. Its gentle song puts anyone who hears it to sleep."
  },
  {
    "species": "JOLTEON",
    "classification": "LIGHTNING",
    "height": 0.79,
    "weight": 24.49,
    "text": "The negatively charged ions generated in its fur create a constant sparking noise."
  },
  {
    "species": "JUMPLUFF",
    "classification": "COTTONWEED",
    "height": 0.79,
    "weight": 3.18,
    "text": "Even in the fiercest wind, it can control its fluff to make its way to any place in the world it wants."
  },
  {
    "species": "JYNX",
    "classification": "HUMANSHAPE",
    "height": 1.4,
    "weight": 40.82,
    "text": "It has several different cry patterns, each of which seems to have its own meaning."
  },
  {
    "species": "KABUTO",
    "classification": "SHELLFISH",
    "height": 0.51,
    "weight": 11.34,
    "text": "Three hundred million years ago, it hid on the sea floor. It also has eyes on its back that glow."
  },
  {
    "species": "KABUTOPS",
    "classification": "SHELLFISH",
    "height": 1.3,
    "weight": 40.37,
    "text": "It was able to swim quickly through the water by compactly folding up its razor-sharp sickles."
  },
  {
    "species": "KADABRA",
    "classification": "PSI",
    "height": 1.3,
    "weight": 56.7,
    "text": "When it closes its eyes, twice as many alpha particles come out of the surface of its body."
  },
  {
    "species": "KAKUNA",
    "classification": "COCOON",
    "height": 0.61,
    "weight": 9.98,
    "text": "Nearly incapable of movement, it leans against stout trees while waiting for its evolution."
  },
  {
    "species": "KANGASKHAN",
    "classification": "PARENT",
    "height": 2.21,
    "weight": 79.83,
    "text": "To avoid crushing the baby it carries in its pouch, it always sleeps standing up."
  },
  {
    "species": "KINGDRA",
    "classification": "DRAGON",
    "height": 1.8,
    "weight": 151.95,
    "text": "It stores energy by sleeping at underwater depths at which no other life forms can survive."
  },
  {
    "species": "KINGLER",
    "classification": "PINCER",
    "height": 1.3,
    "weight": 59.87,
    "text": "Its oversized claw is very powerful, but when it's not in battle, the claw just gets in the way."
  },
  {
    "species": "KOFFING",
    "classification": "POISON GAS",
    "height": 0.61,
    "weight": 0.91,
    "text": "If one gets close enough to it when it expels poisonous gas, the gas swirling inside it can be seen."
  },
  {
    "species": "KRABBY",
    "classification": "RIVER CRAB",
    "height": 0.41,
    "weight": 6.35,
    "text": "If it is unable to find food, it will absorb nutrients by swallowing a mouthful of sand."
  },
  {
    "species": "LANTURN",
    "classification": "LIGHT",
    "height": 1.19,
    "weight": 22.68,
    "text": "This #MON uses the bright part of its body, which changed from a dorsal fin, to lure prey."
  },
  {
    "species": "LAPRAS",
    "classification": "TRANSPORT",
    "height": 2.49,
    "weight": 219.99,
    "text": "This gentle #MON loves to give people rides and provides a very comfortable way to get around."
  },
  {
    "species": "LARVITAR",
    "classification": "ROCK SKIN",
    "height": 0.61,
    "weight": 72.12,
    "text": "Born deep underground, this #MON becomes a pupa after eating enough dirt to make a mountain."
  },
  {
    "species": "LEDIAN",
    "classification": "FIVE STAR",
    "height": 1.4,
    "weight": 35.38,
    "text": "In the daytime when it gets warm, it curls up inside a big leaf and drifts off into a deep slumber."
  },
  {
    "species": "LEDYBA",
    "classification": "FIVE STAR",
    "height": 0.99,
    "weight": 10.89,
    "text": "It is timid and clusters together with others. The fluid secreted by its feet indicates its location."
  },
  {
    "species": "LICKITUNG",
    "classification": "LICKING",
    "height": 1.19,
    "weight": 65.32,
    "text": "It has a tongue that is over 6'6'' long. It uses this long tongue to lick its body clean."
  },
  {
    "species": "LUGIA",
    "classification": "DIVING",
    "height": 5.21,
    "weight": 215.91,
    "text": "It has an incredible ability to calm raging storms. It is said that LUGIA appears when storms start."
  },
  {
    "species": "MACHAMP",
    "classification": "SUPERPOWER",
    "height": 1.6,
    "weight": 130.18,
    "text": "With four arms that react more quickly than it can think, it can execute many punches at once."
  },
  {
    "species": "MACHOKE",
    "classification": "SUPERPOWER",
    "height": 1.5,
    "weight": 70.31,
    "text": "This tough #MON always stays in the zone. Its muscles become thicker after every battle."
  },
  {
    "species": "MACHOP",
    "classification": "SUPERPOWER",
    "height": 0.79,
    "weight": 19.5,
    "text": "It trains by lifting rocks in the mountains. It can even pick up a GRAVELER with ease."
  },
  {
    "species": "MAGBY",
    "classification": "LIVE COAL",
    "height": 0.71,
    "weight": 21.32,
    "text": "It naturally spits an 1100-degree flame. It is said when many appear, it heralds a volcanic eruption."
  },
  {
    "species": "MAGCARGO",
    "classification": "LAVA",
    "height": 0.79,
    "weight": 54.88,
    "text": "Its body is as hot as lava and is always billowing. Flames will occasionally burst from its shell."
  },
  {
    "species": "MAGIKARP",
    "classification": "FISH",
    "height": 0.89,
    "weight": 9.98,
    "text": "This weak and pathetic #MON gets easily pushed along rivers when there are strong currents."
  },
  {
    "species": "MAGMAR",
    "classification": "SPITFIRE",
    "height": 1.3,
    "weight": 44.45,
    "text": "It moves more frequently in hot areas. It can heal itself by dipping its wound into lava."
  },
  {
    "species": "MAGNEMITE",
    "classification": "MAGNET",
    "height": 0.3,
    "weight": 5.9,
    "text": "The electricity emitted by the units on each side of its body cause it to become a strong magnet."
  },
  {
    "species": "MAGNETON",
    "classification": "MAGNET",
    "height": 0.99,
    "weight": 59.87,
    "text": "When many MAGNETON gather together, the resulting magnetic storm disrupts radio waves."
  },
  {
    "species": "MANKEY",
    "classification": "PIG MONKEY",
    "height": 0.51,
    "weight": 28.12,
    "text": "It lives in groups in the treetops. If it loses sight of its group, it becomes infuriated by its loneliness."
  },
  {
    "species": "MANTINE",
    "classification": "KITE",
    "height": 2.11,
    "weight": 219.99,
    "text": "It swims along freely, eating things that swim into its mouth. Its whole body is very coarse."
  },
  {
    "species": "MAREEP",
    "classification": "WOOL",
    "height": 0.61,
    "weight": 7.71,
    "text": "It stores lots of air in its soft fur, allowing it to stay cool in summer and warm in winter."
  },
  {
    "species": "MARILL",
    "classification": "AQUAMOUSE",
    "height": 0.41,
    "weight": 8.62,
    "text": "The fur on its body naturally repels water. It can stay dry, even when it plays in the water."
  },
  {
    "species": "MAROWAK",
    "classification": "BONEKEEPER",
    "height": 0.99,
    "weight": 44.91,
    "text": "Somewhere in the world is a cemetery just for MAROWAK. It gets its bones from those graves."
  },
  {
    "species": "MEGANIUM",
    "classification": "HERB",
    "height": 1.8,
    "weight": 100.7,
    "text": "Anyone who stands beside it becomes refreshed, just as if they were relaxing in a sunny forest."
  },
  {
    "species": "MEOWTH",
    "classification": "SCRATCHCAT",
    "height": 0.41,
    "weight": 4.08,
    "text": "It loves things that sparkle. When it sees a shiny object, the gold coin on its head shines too."
  },
  {
    "species": "METAPOD",
    "classification": "COCOON",
    "height": 0.71,
    "weight": 9.98,
    "text": "This is its preevolved form. At this stage, it can only harden, so it remains motionless to avoid attack."
  },
  {
    "species": "MEW",
    "classification": "NEW SPECIE",
    "height": 0.41,
    "weight": 4.08,
    "text": "Because it can learn any move, some people began research to see if it is the ancestor of all #MON."
  },
  {
    "species": "MEWTWO",
    "classification": "GENETIC",
    "height": 2.01,
    "weight": 122.02,
    "text": "Said to rest quietly in an undiscovered cave, this #MON was created solely for battling."
  },
  {
    "species": "MILTANK",
    "classification": "MILK COW",
    "height": 1.19,
    "weight": 75.3,
    "text": "In order to milk a MILTANK, one must have a knack for rhythmically pulling up and down on its udders."
  },
  {
    "species": "MISDREAVUS",
    "classification": "SCREECH",
    "height": 0.71,
    "weight": 0.91,
    "text": "It loves to watch people it's scared. It frightens them by screaming loudly or appearing suddenly."
  },
  {
    "species": "MOLTRES",
    "classification": "FLAME",
    "height": 2.01,
    "weight": 59.87,
    "text": "Legendary bird #MON. It is said to migrate from the south along with the spring."
  },
  {
    "species": "MR__MIME",
    "classification": "BARRIER",
    "height": 1.3,
    "weight": 54.43,
    "text": "It uses the mysterious power it has in its fingers to solidify air into an invisible wall."
  },
  {
    "species": "MUK",
    "classification": "SLUDGE",
    "height": 1.19,
    "weight": 29.94,
    "text": "As it moves, a very strong poison leaks from it, making the ground there barren for three years."
  },
  {
    "species": "MURKROW",
    "classification": "DARKNESS",
    "height": 0.51,
    "weight": 2.27,
    "text": "It hides any shiny object it finds in a secret location. MURKROW and MEOWTH loot one another's stashes."
  },
  {
    "species": "NATU",
    "classification": "LITTLE BIRD",
    "height": 0.2,
    "weight": 1.81,
    "text": "It is extremely good at climbing tree trunks and likes to eat the new sprouts on the trees."
  },
  {
    "species": "NIDOKING",
    "classification": "DRILL",
    "height": 1.4,
    "weight": 62.14,
    "text": "It uses its thick arms, legs and tail to attack forcefully. Melee combat is its specialty."
  },
  {
    "species": "NIDOQUEEN",
    "classification": "DRILL",
    "height": 1.3,
    "weight": 59.87,
    "text": "The hard scales that cover its strong body serve as excellent protection from any attack."
  },
  {
    "species": "NIDORAN_F",
    "classification": "POISON PIN",
    "height": 0.41,
    "weight": 6.8,
    "text": "Small and very docile, it protects itself with its small, poisonous horn when attacked."
  },
  {
    "species": "NIDORAN_M",
    "classification": "POISON PIN",
    "height": 0.51,
    "weight": 9.07,
    "text": "It constantly moves its large ears in many directions in order to detect danger right away."
  },
  {
    "species": "NIDORINA",
    "classification": "POISON PIN",
    "height": 0.79,
    "weight": 19.96,
    "text": "It has a docile nature. If it is threatened with attack, it raises the barbs that are all over its body."
  },
  {
    "species": "NIDORINO",
    "classification": "POISON PIN",
    "height": 0.89,
    "weight": 19.5,
    "text": "It is easily agitated and uses its horn for offense as soon as it notices an attacker."
  },
  {
    "species": "NINETALES",
    "classification": "FOX",
    "height": 1.09,
    "weight": 19.96,
    "text": "It is said to live a thousand years, and each of its tails is loaded with supernatural powers."
  },
  {
    "species": "NOCTOWL",
    "classification": "OWL",
    "height": 1.6,
    "weight": 40.82,
    "text": "Its extremely soft feathers make no sound in flight. It silently sneaks up on prey without being detected."
  },
  {
    "species": "OCTILLERY",
    "classification": "JET",
    "height": 0.89,
    "weight": 28.58,
    "text": "Its instinct is to bury itself in holes. It often steals the nesting holes of others to sleep in them."
  },
  {
    "species": "ODDISH",
    "classification": "WEED",
    "height": 0.51,
    "weight": 5.44,
    "text": "During the day, it stays in the cold underground to avoid the sun. It grows by bathing in moonlight."
  },
  {
    "species": "OMANYTE",
    "classification": "SPIRAL",
    "height": 0.41,
    "weight": 7.71,
    "text": "In prehistoric times, it swam on the sea floor, eating plankton. Its fossils are sometimes found."
  },
  {
    "species": "OMASTAR",
    "classification": "SPIRAL",
    "height": 0.99,
    "weight": 34.93,
    "text": "Its heavy shell allowed it to reach only nearby food. This could be the reason it is extinct."
  },
  {
    "species": "ONIX",
    "classification": "ROCK SNAKE",
    "height": 8.79,
    "weight": 210.01,
    "text": "As it digs through the ground, it absorbs many hard objects. This is what makes its body so solid."
  },
  {
    "species": "PARAS",
    "classification": "MUSHROOM",
    "height": 0.3,
    "weight": 5.44,
    "text": "The tochukaso growing on this #MON's back orders it to extract juice from tree trunks."
  },
  {
    "species": "PARASECT",
    "classification": "MUSHROOM",
    "height": 0.99,
    "weight": 29.48,
    "text": "When nothing's left to extract from the bug, the mushrooms on its back leave spores on the bug's egg."
  },
  {
    "species": "PERSIAN",
    "classification": "CLASSY CAT",
    "height": 0.99,
    "weight": 32.21,
    "text": "Behind its lithe, elegant appearance lies a barbaric side. It will tear apart its prey on a mere whim."
  },
  {
    "species": "PHANPY",
    "classification": "LONG NOSE",
    "height": 0.51,
    "weight": 33.57,
    "text": "During the deserted morning hours, it comes ashore where it deftly uses its trunk to take a shower."
  },
  {
    "species": "PICHU",
    "classification": "TINY MOUSE",
    "height": 0.3,
    "weight": 1.81,
    "text": "It is unskilled at storing electric power. Any kind of shock causes it to discharge energy spontaneously."
  },
  {
    "species": "PIDGEOT",
    "classification": "BIRD",
    "height": 1.5,
    "weight": 39.46,
    "text": "Its outstanding vision allows it to spot splashing MAGIKARP, even while flying at 3300 feet."
  },
  {
    "species": "PIDGEOTTO",
    "classification": "BIRD",
    "height": 1.09,
    "weight": 29.94,
    "text": "It slowly flies in a circular pattern, all the while keeping a sharp lookout for prey."
  },
  {
    "species": "PIDGEY",
    "classification": "TINY BIRD",
    "height": 0.3,
    "weight": 1.81,
    "text": "It rapidly flaps its wings in the grass, stirring up a dust cloud that drives insect prey out into the open."
  },
  {
    "species": "PIKACHU",
    "classification": "MOUSE",
    "height": 0.41,
    "weight": 5.9,
    "text": "When it is angered, it immediately discharges the energy stored in the pouches in its cheeks."
  },
  {
    "species": "PILOSWINE",
    "classification": "SWINE",
    "height": 1.09,
    "weight": 55.79,
    "text": "Although its legs are short, its rugged hooves prevent it from slipping, even on icy ground."
  },
  {
    "species": "PINECO",
    "classification": "BAGWORM",
    "height": 0.61,
    "weight": 7.26,
    "text": "It spits out a fluid that it uses to glue tree bark to its body. The fluid hardens when it touches air."
  },
  {
    "species": "PINSIR",
    "classification": "STAGBEETLE",
    "height": 1.5,
    "weight": 54.88,
    "text": "When the temperature drops at night, it sleeps on treetops or among roots where it is well hidden."
  },
  {
    "species": "POLITOED",
    "classification": "FROG",
    "height": 1.09,
    "weight": 34.02,
    "text": "When it expands its throat to croak out a tune, nearby POLIWAG and POLIWHIRL gather immediately."
  },
  {
    "species": "POLIWAG",
    "classification": "TADPOLE",
    "height": 0.61,
    "weight": 12.25,
    "text": "The swirl on its belly is its insides showing through the skin. It looks clearer after it eats."
  },
  {
    "species": "POLIWHIRL",
    "classification": "TADPOLE",
    "height": 0.99,
    "weight": 19.96,
    "text": "Though it is skilled at walking, it prefers to live underwater where there is less danger."
  },
  {
    "species": "POLIWRATH",
    "classification": "TADPOLE",
    "height": 1.3,
    "weight": 53.98,
    "text": "It can use its well-developed arms and legs to run on the surface of the water for a split second."
  },
  {
    "species": "PONYTA",
    "classification": "FIRE HORSE",
    "height": 0.99,
    "weight": 29.94,
    "text": "Training by jumping over grass that grows longer every day has made it a world-class jumper."
  },
  {
    "species": "PORYGON",
    "classification": "VIRTUAL",
    "height": 0.79,
    "weight": 36.29,
    "text": "An artificial #MON created due to extensive research, it can perform only what is in its program."
  },
  {
    "species": "PORYGON2",
    "classification": "VIRTUAL",
    "height": 0.61,
    "weight": 32.66,
    "text": "This manmade #MON evolved from the latest technology. It may have unprogrammed reactions."
  },
  {
    "species": "PRIMEAPE",
    "classification": "PIG MONKEY",
    "height": 0.99,
    "weight": 32.21,
    "text": "It will beat up anyone who makes it mad, even if it has to chase them until the end of the world."
  },
  {
    "species": "PSYDUCK",
    "classification": "DUCK",
    "height": 0.79,
    "weight": 19.5,
    "text": "The only time it can use its psychic power is when its sleeping brain cells happen to wake."
  },
  {
    "species": "PUPITAR",
    "classification": "HARD SHELL",
    "height": 1.19,
    "weight": 151.95,
    "text": "It will not stay still, even while it's a pupa. It already has arms and legs under its solid shell."
  },
  {
    "species": "QUAGSIRE",
    "classification": "WATER FISH",
    "height": 1.4,
    "weight": 74.84,
    "text": "Its body is always slimy. It often bangs its head on the river bottom as it swims but seems not to care."
  },
  {
    "species": "QUILAVA",
    "classification": "VOLCANO",
    "height": 0.89,
    "weight": 19.05,
    "text": "Before battle, it turns its back on its opponent to demonstrate how ferociously its fire blazes."
  },
  {
    "species": "QWILFISH",
    "classification": "BALLOON",
    "height": 0.51,
    "weight": 4.08,
    "text": "When faced with a larger opponent, it swallows as much water as it can to match the opponent's size."
  },
  {
    "species": "RAICHU",
    "classification": "MOUSE",
    "height": 0.79,
    "weight": 29.94,
    "text": "If its electric pouches run empty, it raises its tail to gather electricity from the atmosphere."
  },
  {
    "species": "RAIKOU",
    "classification": "THUNDER",
    "height": 1.91,
    "weight": 177.81,
    "text": "This rough #MON stores energy inside its body, then sweeps across the land, shooting off electricity."
  },
  {
    "species": "RAPIDASH",
    "classification": "FIRE HORSE",
    "height": 1.7,
    "weight": 94.8,
    "text": "It just loves to gallop. The faster it goes, the longer the swaying flames of its mane will become."
  },
  {
    "species": "RATICATE",
    "classification": "RAT",
    "height": 0.71,
    "weight": 18.6,
    "text": "The webs on its hind legs enable it to cross rivers. It searches wide areas for food."
  },
  {
    "species": "RATTATA",
    "classification": "RAT",
    "height": 0.3,
    "weight": 3.63,
    "text": "This #MON's impressive vitality allows it to live anywhere. It also multiplies very quickly."
  },
  {
    "species": "REMORAID",
    "classification": "JET",
    "height": 0.61,
    "weight": 11.79,
    "text": "To escape from an attacker, it may shoot water out of its mouth, then use that force to swim backward."
  },
  {
    "species": "RHYDON",
    "classification": "DRILL",
    "height": 1.91,
    "weight": 120.2,
    "text": "By lightly spinning its drilllike horn, it can easily shatter even a diamond in the rough."
  },
  {
    "species": "RHYHORN",
    "classification": "SPIKES",
    "height": 0.99,
    "weight": 115.21,
    "text": "It can remember only one thing at a time. Once it starts rushing, it forgets why it started."
  },
  {
    "species": "SANDSHREW",
    "classification": "MOUSE",
    "height": 0.61,
    "weight": 11.79,
    "text": "It prefers dry, sandy places because it uses the sand to protect itself when threatened."
  },
  {
    "species": "SANDSLASH",
    "classification": "MOUSE",
    "height": 0.99,
    "weight": 29.48,
    "text": "Adept at climbing trees, it rolls into a spiny ball, then attacks its enemies from above."
  },
  {
    "species": "SCIZOR",
    "classification": "SCISSORS",
    "height": 1.8,
    "weight": 117.93,
    "text": "This #MON's pincers, which contain steel, can crush any hard object it gets a hold of into bits."
  },
  {
    "species": "SCYTHER",
    "classification": "MANTIS",
    "height": 1.5,
    "weight": 55.79,
    "text": "It's very proud of its speed. It moves so fast that its opponent does not even know what knocked it down."
  },
  {
    "species": "SEADRA",
    "classification": "DRAGON",
    "height": 1.19,
    "weight": 24.95,
    "text": "The male raises the young. If it is approached, it uses its toxic spikes to fend off the intruder."
  },
  {
    "species": "SEAKING",
    "classification": "GOLDFISH",
    "height": 1.3,
    "weight": 39.01,
    "text": "When autumn comes, the males patrol the area around their nests in order to protect their offspring."
  },
  {
    "species": "SEEL",
    "classification": "SEA LION",
    "height": 1.09,
    "weight": 89.81,
    "text": "The light blue fur that covers it keeps it protected against the cold. It loves icebergfilled oceans."
  },
  {
    "species": "SENTRET",
    "classification": "SCOUT",
    "height": 0.79,
    "weight": 5.9,
    "text": "When acting as a lookout, it warns others of danger by screeching and hitting the ground with its tail."
  },
  {
    "species": "SHELLDER",
    "classification": "BIVALVE",
    "height": 0.3,
    "weight": 4.08,
    "text": "Clamping on to an opponent reveals its vulnerable parts, so it uses this move only as a last resort."
  },
  {
    "species": "SHUCKLE",
    "classification": "MOLD",
    "height": 0.61,
    "weight": 20.41,
    "text": "The fluid secreted by its toes carves holes in rocks for nesting and can be mixed with BERRIES to make a drink."
  },
  {
    "species": "SKARMORY",
    "classification": "ARMOR BIRD",
    "height": 1.7,
    "weight": 50.35,
    "text": "The feathers that it sheds are very sharp. It is said that people once used the feathers as swords."
  },
  {
    "species": "SKIPLOOM",
    "classification": "COTTONWEED",
    "height": 0.61,
    "weight": 0.91,
    "text": "As soon as it rains, it closes its flower and hides in the shade of a tree to avoid getting wet."
  },
  {
    "species": "SLOWBRO",
    "classification": "HERMITCRAB",
    "height": 1.6,
    "weight": 78.47,
    "text": "An attached SHELLDER won't let go because of the tasty flavor that oozes out of its tail."
  },
  {
    "species": "SLOWKING",
    "classification": "ROYAL",
    "height": 2.01,
    "weight": 79.38,
    "text": "Every time it yawns, SHELLDER injects more poison into it. The poison makes it more intelligent."
  },
  {
    "species": "SLOWPOKE",
    "classification": "DOPEY",
    "height": 1.19,
    "weight": 35.83,
    "text": "It is always so absent-minded that it won't react, even if its flavorful tail is bitten."
  },
  {
    "species": "SLUGMA",
    "classification": "LAVA",
    "height": 0.71,
    "weight": 34.93,
    "text": "These group together in areas that are hotter than normal. If it cools off, its skin hardens."
  },
  {
    "species": "SMEARGLE",
    "classification": "PAINTER",
    "height": 1.19,
    "weight": 58.06,
    "text": "The color of the mysterious fluid secreted from its tail is predetermined for each SMEARGLE."
  },
  {
    "species": "SMOOCHUM",
    "classification": "KISS",
    "height": 0.41,
    "weight": 5.9,
    "text": "The sensitivity of its lips develops most quickly. It uses them to try to identify unknown objects."
  },
  {
    "species": "SNEASEL",
    "classification": "SHARP CLAW",
    "height": 0.89,
    "weight": 28.12,
    "text": "This cunning #MON hides under the cover of darkness, waiting to attack its prey."
  },
  {
    "species": "SNORLAX",
    "classification": "SLEEPING",
    "height": 2.11,
    "weight": 459.94,
    "text": "This #MON's stomach is so strong, even eating moldy or rotten food will not affect it."
  },
  {
    "species": "SNUBBULL",
    "classification": "FAIRY",
    "height": 0.61,
    "weight": 7.71,
    "text": "In truth, it is a cowardly #MON. It growls eagerly in order to hide its fear from its opponent."
  },
  {
    "species": "SPEAROW",
    "classification": "TINY BIRD",
    "height": 0.3,
    "weight": 1.81,
    "text": "To protect its territory, it flies around ceaselessly, making highpitched cries."
  },
  {
    "species": "SPINARAK",
    "classification": "STRINGSPIT",
    "height": 0.51,
    "weight": 8.62,
    "text": "If prey becomes ensnared in its nest of spun string, it waits motionlessly until it becomes dark."
  },
  {
    "species": "SQUIRTLE",
    "classification": "TINYTURTLE",
    "height": 0.51,
    "weight": 9.07,
    "text": "When it feels threatened, it draws its legs inside its shell and sprays water from its mouth."
  },
  {
    "species": "STANTLER",
    "classification": "BIG HORN",
    "height": 1.4,
    "weight": 71.21,
    "text": "The round balls found on the fallen antlers can be ground into a powder that aids in sleeping."
  },
  {
    "species": "STARMIE",
    "classification": "MYSTERIOUS",
    "height": 1.09,
    "weight": 79.83,
    "text": "It is said that it uses the sevencolored core of its body to send electric waves into outer space."
  },
  {
    "species": "STARYU",
    "classification": "STARSHAPE",
    "height": 0.79,
    "weight": 34.47,
    "text": "When the stars twinkle at night, it floats up from the sea floor, and its body's center core flickers."
  },
  {
    "species": "STEELIX",
    "classification": "IRON SNAKE",
    "height": 9.19,
    "weight": 400.07,
    "text": "The many small metal particles that cover this #MON's body reflect bright light well."
  },
  {
    "species": "SUDOWOODO",
    "classification": "IMITATION",
    "height": 1.19,
    "weight": 38.1,
    "text": "If a tree branch shakes when there is no wind, it's a SUDOWOODO, not a tree. It hides from the rain."
  },
  {
    "species": "SUICUNE",
    "classification": "AURORA",
    "height": 2.01,
    "weight": 186.88,
    "text": "This divine #MON blows around the world, always in search of a pure reservoir."
  },
  {
    "species": "SUNFLORA",
    "classification": "SUN",
    "height": 0.79,
    "weight": 8.62,
    "text": "As the hot season approaches, the petals on this #MON's face become more vivid and lively."
  },
  {
    "species": "SUNKERN",
    "classification": "SEED",
    "height": 0.3,
    "weight": 1.81,
    "text": "It is very weak. Its only means of defense is to shake its leaves desperately at its attacker."
  },
  {
    "species": "SWINUB",
    "classification": "PIG",
    "height": 0.41,
    "weight": 6.35,
    "text": "It uses the tip of its nose to dig for food. Its nose is so tough that even frozen ground poses no problem."
  },
  {
    "species": "TANGELA",
    "classification": "VINE",
    "height": 0.99,
    "weight": 34.93,
    "text": "During battle, it constantly moves the vines that cover its body in order to annoy its opponent."
  },
  {
    "species": "TAUROS",
    "classification": "WILD BULL",
    "height": 1.4,
    "weight": 88.45,
    "text": "These violent #MON fight with other members of their herd in order to prove their strength."
  },
  {
    "species": "TEDDIURSA",
    "classification": "LITTLE BEAR",
    "height": 0.61,
    "weight": 8.62,
    "text": "It always licks honey. Its palm tastes sweet because of all the honey it has absorbed."
  },
  {
    "species": "TENTACOOL",
    "classification": "JELLYFISH",
    "height": 0.89,
    "weight": 45.36,
    "text": "As it floats along on the waves, it uses its toxic feelers to stab anything it touches."
  },
  {
    "species": "TENTACRUEL",
    "classification": "JELLYFISH",
    "height": 1.6,
    "weight": 54.88,
    "text": "When its 80 feelers absorb water, it stretches to become like a net to entangle its prey."
  },
  {
    "species": "TOGEPI",
    "classification": "SPIKE BALL",
    "height": 0.3,
    "weight": 1.36,
    "text": "It is considered to be a symbol of good luck. Its shell is said to be filled with happiness."
  },
  {
    "species": "TOGETIC",
    "classification": "HAPPINESS",
    "height": 0.61,
    "weight": 3.18,
    "text": "Although it does not flap its wings very much, it can stay up in the air as it tags along after its trainer."
  },
  {
    "species": "TOTODILE",
    "classification": "BIG JAW",
    "height": 0.61,
    "weight": 9.53,
    "text": "This rough critter chomps at any moving object it sees. Turning your back on it is not recommended."
  },
  {
    "species": "TYPHLOSION",
    "classification": "VOLCANO",
    "height": 1.7,
    "weight": 79.38,
    "text": "When heat from its body causes the air around it to shimmer, this is a sign that it is ready to battle."
  },
  {
    "species": "TYRANITAR",
    "classification": "ARMOR",
    "height": 2.01,
    "weight": 201.85,
    "text": "In just one of its mighty hands, it has the power to make the ground shake and mountains crumble."
  },
  {
    "species": "TYROGUE",
    "classification": "SCUFFLE",
    "height": 0.71,
    "weight": 20.87,
    "text": "To brush up on its fighting skills, it will challenge anyone. It has a very strong competitive spirit."
  },
  {
    "species": "UMBREON",
    "classification": "MOONLIGHT",
    "height": 0.99,
    "weight": 27.22,
    "text": "On the night of a full moon, or when it gets excited, the ring patterns on its body glow yellow."
  },
  {
    "species": "UNOWN",
    "classification": "SYMBOL",
    "height": 0.51,
    "weight": 4.99,
    "text": "Because different types of UNOWN exist, it is said that they must have a variety of abilities."
  },
  {
    "species": "URSARING",
    "classification": "HIBERNANT",
    "height": 1.8,
    "weight": 125.65,
    "text": "Although it has a large body, it is quite skilled at climbing trees. It eats and sleeps in the treetops."
  },
  {
    "species": "VAPOREON",
    "classification": "BUBBLE JET",
    "height": 0.99,
    "weight": 29.03,
    "text": "As it uses the fins on the tip of its tail to swim, it blends with the water perfectly."
  },
  {
    "species": "VENOMOTH",
    "classification": "POISONMOTH",
    "height": 1.5,
    "weight": 12.7,
    "text": "The scales it scatters will paralyze anyone who touches them, making that person unable to stand."
  },
  {
    "species": "VENONAT",
    "classification": "INSECT",
    "height": 0.99,
    "weight": 29.94,
    "text": "The small bugs it eats appear only at night, so it sleeps in a hole in a tree until night falls."
  },
  {
    "species": "VENUSAUR",
    "classification": "SEED",
    "height": 2.01,
    "weight": 100.24,
    "text": "As it warms itself and absorbs the sunlight, its flower petals release a pleasant fragrance."
  },
  {
    "species": "VICTREEBEL",
    "classification": "FLYCATCHER",
    "height": 1.7,
    "weight": 15.42,
    "text": "Once ingested into this #MON's body, even the hardest object will melt into nothing."
  },
  {
    "species": "VILEPLUME",
    "classification": "FLOWER",
    "height": 1.19,
    "weight": 18.6,
    "text": "By shaking its big petals, it scatters toxic pollen into the air, turning the air yellow."
  },
  {
    "species": "VOLTORB",
    "classification": "BALL",
    "height": 0.51,
    "weight": 10.43,
    "text": "During the study of this #MON, it was discovered that its components are not found in nature."
  },
  {
    "species": "VULPIX",
    "classification": "FOX",
    "height": 0.61,
    "weight": 9.98,
    "text": "As its body grows larger, its six warm tails become more beautiful, with a more luxurious coat of fur."
  },
  {
    "species": "WARTORTLE",
    "classification": "TURTLE",
    "height": 0.99,
    "weight": 22.68,
    "text": "Its long, furry tail is a symbol of longevity, making it quite popular among older people."
  },
  {
    "species": "WEEDLE",
    "classification": "HAIRY BUG",
    "height": 0.3,
    "weight": 3.18,
    "text": "The barb on top of its head secretes a strong poison. It uses this toxic barb to protect itself."
  },
  {
    "species": "WEEPINBELL",
    "classification": "FLYCATCHER",
    "height": 0.99,
    "weight": 6.35,
    "text": "When it's hungry, it swings its razor-sharp leaves, slicing up any unlucky object nearby for food."
  },
  {
    "species": "WEEZING",
    "classification": "POISON GAS",
    "height": 1.19,
    "weight": 9.53,
    "text": "When it inhales poisonous gases from garbage, its body expands, and its insides smell much worse."
  },
  {
    "species": "WIGGLYTUFF",
    "classification": "BALLOON",
    "height": 0.99,
    "weight": 11.79,
    "text": "The rich, fluffy fur that covers its body feels so good that anyone who feels it can't stop touching it."
  },
  {
    "species": "WOBBUFFET",
    "classification": "PATIENT",
    "height": 1.3,
    "weight": 28.58,
    "text": "In order to conceal its black tail, it lives in a dark cave and only moves about at night."
  },
  {
    "species": "WOOPER",
    "classification": "WATER FISH",
    "height": 0.41,
    "weight": 8.62,
    "text": "A mucous membrane covers its body. Touching it barehanded will cause a shooting pain."
  },
  {
    "species": "XATU",
    "classification": "MYSTIC",
    "height": 1.5,
    "weight": 14.97,
    "text": "Once it begins to meditate at sunrise, the entire day will pass before it will move again."
  },
  {
    "species": "YANMA",
    "classification": "CLEAR WING",
    "height": 1.19,
    "weight": 38.1,
    "text": "It can see in all directions without moving its big eyes, helping it spot attackers and food right away."
  },
  {
    "species": "ZAPDOS",
    "classification": "ELECTRIC",
    "height": 1.6,
    "weight": 52.62,
    "text": "Legendary bird #MON. They say lightning caused by the flapping of its wings causes summer storms."
  },
  {
    "species": "ZUBAT",
    "classification": "BAT",
    "height": 0.79,
    "weight": 7.71,
    "text": "During the day, it gathers with others and hangs from the ceilings of old buildings and caves."
  }
];
