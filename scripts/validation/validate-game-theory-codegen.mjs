import fs from 'node:fs';

const generated = fs.readFileSync('open-game-theory-ontology/fixtures/generated/rust/game_components.rs', 'utf8');
const ontologyCargo = fs.readFileSync('crates/kairo-ecs-game-ontology/Cargo.toml', 'utf8');
const gameCargo = fs.readFileSync('crates/kairo-ecs-game-theory/Cargo.toml', 'utf8');
const gameLib = fs.readFileSync('crates/kairo-ecs-game-theory/src/lib.rs', 'utf8');
const manifest = JSON.parse(fs.readFileSync('open-game-theory-ontology/fixtures/generated/rust/manifest.json', 'utf8'));

const failures = [];
function requireText(source, needle, label) {
  if (!source.includes(needle)) failures.push(label + ' missing: ' + needle);
}

requireText(ontologyCargo, 'codegen = []', 'ontology crate feature gate');
requireText(gameCargo, 'generated-components = []', 'game theory crate feature gate');
requireText(gameLib, 'include!("../../../open-game-theory-ontology/fixtures/generated/rust/game_components.rs")', 'generated fixture include');
requireText(generated, 'pub struct Entity(pub u64);', 'Entity ID wrapper');
requireText(generated, 'pub transition_to: Vec<Entity>,', 'Action transition edge');
requireText(generated, 'pub has_action: Vec<Entity>,', 'DecisionNode action edge');
requireText(generated, 'pub in_information_set: Vec<Entity>,', 'DecisionNode information-set edge');
requireText(generated, 'pub has_player: Vec<Entity>,', 'Game player edge');
requireText(generated, 'pub has_payoff_matrix: Vec<Entity>,', 'Game payoff edge');
requireText(generated, 'pub has_strategy: Vec<Entity>,', 'Player strategy edge');
requireText(generated, 'pub has_utility: Vec<Entity>,', 'PayoffMatrix utility edge');

for (const forbidden of ['Box<', '*const', '*mut', 'Rc<', 'Arc<']) {
  if (generated.includes(forbidden)) failures.push('generated component fixture contains pointer topology token: ' + forbidden);
}
if (!Array.isArray(manifest.source_fixtures) || manifest.source_fixtures.length < 4) failures.push('manifest must record all ontology source_fixtures');
if (manifest.generated_fixture !== 'game_components.rs') failures.push('manifest generated_fixture must be game_components.rs');

if (failures.length) {
  console.error(failures.join('\n'));
  process.exit(1);
}
console.log('game theory codegen validation passed');
