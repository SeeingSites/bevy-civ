// Expansion packs: nations that can be led from 4000 BC (on the Whole Earth or a New World) and
// scenarios set in their periods. build.cjs writes each pack's peoples to assets/expansions/<id>.json
// (their homes snapped onto the Whole Earth map) and its scenarios to assets/scenarios/<id>.json.
// Places are [name, longitude, latitude, population]; units are [type, count, index of city].

// What each technology needs (as in src/tech.rs), so a period's knowledge can be given as its
// newest discoveries.
const PREREQ = {
  'Writing': ['Alphabet'], 'Code of Laws': ['Writing'], 'Mysticism': ['Pottery'], 'Seafaring': ['Pottery'],
  'Iron Working': ['Bronze Working'], 'Mathematics': ['Writing', 'Masonry'], 'Horseback Riding': ['Animal Husbandry'],
  'Construction': ['Masonry'], 'Philosophy': ['Mysticism', 'Writing'], 'Monarchy': ['Code of Laws'], 'Currency': ['Code of Laws'],
  'Engineering': ['Mathematics', 'Construction'], 'Gunpowder': ['Iron Working', 'Engineering'], 'Printing Press': ['Writing'],
  'Democracy': ['Philosophy', 'Printing Press'], 'Steam Power': ['Gunpowder'], 'Industrialization': ['Steam Power', 'Currency'],
  'Refining': ['Industrialization'], 'Replaceable Parts': ['Industrialization'], 'Motorized Transport': ['Refining', 'Replaceable Parts'],
  'The Republic': ['Code of Laws', 'Writing'], 'Theology': ['Philosophy', 'Monarchy'], 'Fundamentalism': ['Theology', 'Gunpowder'],
  'Communism': ['Philosophy', 'Industrialization'], 'Sanitation': ['Engineering', 'Industrialization'], 'Electricity': ['Steam Power', 'Mathematics'],
  'Computers': ['Electronics', 'Replaceable Parts'], 'Banking': ['Currency', 'The Republic'], 'Economics': ['Banking', 'Printing Press'],
  'Astronomy': ['Mysticism', 'Mathematics'], 'Chivalry': ['Monarchy', 'Horseback Riding'], 'Navigation': ['Seafaring', 'Astronomy'],
  'Theory of Gravity': ['Astronomy', 'Printing Press'], 'Medicine': ['Philosophy', 'Currency'], 'Steel': ['Electricity', 'Industrialization'],
  'Electronics': ['Electricity', 'Engineering'], 'Flight': ['Refining', 'Theory of Gravity'], 'Advanced Flight': ['Flight', 'Electronics'],
  'Mass Production': ['Motorized Transport', 'Economics'],
};
const BASICS = ['Pottery', 'Animal Husbandry', 'Mining', 'Bronze Working', 'Masonry', 'Alphabet', 'The Wheel'];
/** Every technology needed for `newest`, and those. */
function knowing(...newest) {
  const out = new Set(BASICS);
  const add = (t) => { if (out.has(t) && !newest.includes(t)) return; out.add(t); (PREREQ[t] || []).forEach(add); };
  newest.forEach(add);
  return [...out];
}

const ENLIGHTENMENT = knowing('Navigation', 'Theory of Gravity', 'Economics', 'Medicine', 'Theology', 'Chivalry', 'Gunpowder', 'Monarchy');
const REVOLUTION = knowing(...['The Republic', 'Navigation', 'Theory of Gravity', 'Economics', 'Medicine', 'Theology', 'Chivalry', 'Gunpowder', 'Democracy']);
const GREAT_WAR = knowing('The Republic', 'Steel', 'Replaceable Parts', 'Refining', 'Sanitation', 'Economics', 'Democracy', 'Theology', 'Medicine', 'Navigation', 'Theory of Gravity', 'Chivalry');
const WORLD_WAR = knowing('Fundamentalism', 'The Republic', 'Advanced Flight', 'Mass Production', 'Steel', 'Sanitation', 'Democracy', 'Theology', 'Medicine', 'Navigation', 'Chivalry');

// Peoples of the Americas before contact: farming, building, writing their laws.
const FIRST_NATIONS = knowing('Mysticism', 'Code of Laws', 'Horseback Riding', 'Iron Working', 'Construction');

const EARTH = { lon0: -169, lon1: 191, latTop: 73, latBottom: -56, cols: 90, landThreshold: 2 };

const packs = [
  {
    id: 'revolution',
    name: 'Age of Revolution',
    blurb: 'The United States, France, Spain, Prussia and Austria as peoples to lead from 4000 BC, and two scenarios: The American Revolution (1775) and The Napoleonic Wars (1805).',
    peoples: [
      { name: 'the United States', adjective: 'American', leader: 'Washington', portrait: 22, aggression: 0.5, home: ['Philadelphia', -75.2, 40.0],
        cityNames: ['Boston', 'New York', 'Charleston', 'Baltimore', 'Richmond', 'Pittsburgh', 'Chicago', 'New Orleans'] },
      { name: 'France', adjective: 'French', leader: 'Napoleon', portrait: 14, aggression: 0.6, home: ['Paris', 2.3, 48.9],
        cityNames: ['Lyon', 'Marseille', 'Bordeaux', 'Toulouse', 'Rouen', 'Nantes', 'Strasbourg', 'Lille'] },
      { name: 'Spain', adjective: 'Spanish', leader: 'Isabella', portrait: 13, aggression: 0.55, home: ['Madrid', -3.7, 40.4],
        cityNames: ['Seville', 'Barcelona', 'Toledo', 'Valencia', 'Cadiz', 'Granada', 'Zaragoza', 'Bilbao'] },
      { name: 'Prussia', adjective: 'Prussian', leader: 'Frederick', portrait: 23, aggression: 0.65, home: ['Berlin', 13.4, 52.5],
        cityNames: ['Königsberg', 'Breslau', 'Magdeburg', 'Potsdam', 'Danzig', 'Stettin', 'Cologne', 'Halle'] },
      { name: 'Austria', adjective: 'Austrian', leader: 'Joseph', portrait: 24, aggression: 0.45, home: ['Vienna', 16.4, 48.2],
        cityNames: ['Prague', 'Budapest', 'Graz', 'Innsbruck', 'Salzburg', 'Linz', 'Trieste', 'Brno'] },
    ],
  },
  {
    id: 'worldwars',
    name: 'World Wars',
    blurb: 'Germany, Italy, Russia and the Ottomans as peoples to lead from 4000 BC, and two scenarios: The Great War (1914) and The Second World War (1939).',
    peoples: [
      { name: 'Germany', adjective: 'German', leader: 'Bismarck', portrait: 25, aggression: 0.65, home: ['Berlin', 13.4, 52.5],
        cityNames: ['Hamburg', 'Munich', 'Cologne', 'Frankfurt', 'Leipzig', 'Dresden', 'Stuttgart', 'Bremen'] },
      { name: 'Italy', adjective: 'Italian', leader: 'Victor Emmanuel', portrait: 26, aggression: 0.5, home: ['Rome', 12.5, 41.9], replaces: 'Rome',
        cityNames: ['Milan', 'Naples', 'Turin', 'Florence', 'Venice', 'Genoa', 'Palermo', 'Bologna'] },
      { name: 'Russia', adjective: 'Russian', leader: 'Peter', portrait: 16, aggression: 0.6, home: ['Moscow', 37.6, 55.8],
        cityNames: ['Saint Petersburg', 'Kiev', 'Kazan', 'Novgorod', 'Smolensk', 'Odessa', 'Tula', 'Novosibirsk'] },
      { name: 'the Ottomans', adjective: 'Ottoman', leader: 'Suleiman', portrait: 15, aggression: 0.7, home: ['Bursa', 29.1, 40.2],
        cityNames: ['Constantinople', 'Edirne', 'Ankara', 'Smyrna', 'Konya', 'Trebizond', 'Damascus', 'Aleppo'] },
    ],
  },
  {
    id: 'modern',
    name: 'Modern Nations',
    blurb: 'The United Kingdom, Canada, Mexico, Brazil and Australia as peoples to lead from 4000 BC, and The Modern World, a game that starts in 2025.',
    peoples: [
      { name: 'the United Kingdom', adjective: 'British', leader: 'The Prime Minister', portrait: 20, aggression: 0.45, home: ['London', -0.1, 51.5], replaces: 'England',
        cityNames: ['Manchester', 'Birmingham', 'Glasgow', 'Edinburgh', 'Liverpool', 'Cardiff', 'Belfast', 'Leeds'] },
      { name: 'Canada', adjective: 'Canadian', leader: 'The Prime Minister', portrait: 21, aggression: 0.3, home: ['Ottawa', -75.7, 45.4],
        cityNames: ['Toronto', 'Montreal', 'Vancouver', 'Calgary', 'Edmonton', 'Winnipeg', 'Quebec City', 'Halifax'] },
      { name: 'Mexico', adjective: 'Mexican', leader: 'The President', portrait: 18, aggression: 0.4, home: ['Mexico City', -99.1, 19.4], replaces: 'the Aztecs',
        cityNames: ['Guadalajara', 'Monterrey', 'Puebla', 'Tijuana', 'Mérida', 'León', 'Veracruz', 'Oaxaca'] },
      { name: 'Brazil', adjective: 'Brazilian', leader: 'The President', portrait: 19, aggression: 0.4, home: ['Brasília', -47.9, -15.8],
        cityNames: ['São Paulo', 'Rio de Janeiro', 'Salvador', 'Fortaleza', 'Belo Horizonte', 'Manaus', 'Recife', 'Porto Alegre'] },
      { name: 'Australia', adjective: 'Australian', leader: 'The Prime Minister', portrait: 20, aggression: 0.35, home: ['Canberra', 149.1, -35.3],
        cityNames: ['Sydney', 'Melbourne', 'Brisbane', 'Perth', 'Adelaide', 'Hobart', 'Darwin', 'Cairns'] },
    ],
  },
];

const ATLANTIC = { lon0: -100, lon1: 12, latTop: 60, latBottom: 8, cols: 72, landThreshold: 3 };
const EUROPE = { lon0: -11, lon1: 46, latTop: 61, latBottom: 33.5, cols: 64, landThreshold: 3 };
const GREAT_WAR_MAP = { lon0: -11, lon1: 50, latTop: 61, latBottom: 28.5, cols: 66, landThreshold: 3 };
const WORLD = { ...EARTH, cols: 110 };

const scenarios = [
  {
    id: 'revolution', expansion: 'revolution',
    name: 'The American Revolution',
    blurb: '1775. Thirteen colonies have taken up arms against the British crown, and the Iroquois fight on Britain\'s side. France is the colonies\' ally; Spain watches from Havana and Madrid.',
    startYear: 1775, yearsPerTurn: 1, known: true, map: ATLANTIC,
    civs: [
      { name: 'the United States', adjective: 'American', leader: 'Washington', portrait: 22, aggression: 0.5, gold: 40, government: 'Republic', techs: REVOLUTION,
        cities: [['Philadelphia', -75.2, 40.0, 5], ['Boston', -71.1, 42.4, 4], ['New York', -74.0, 40.7, 4], ['Williamsburg', -76.7, 37.3, 3], ['Charleston', -79.9, 32.8, 3]],
        units: [['Musketman', 2, 0], ['Musketman', 2, 1], ['Cannon', 1, 0], ['Cavalry', 1, 0], ['Musketman', 1, 3], ['Musketman', 1, 4], ['Worker', 1, 0], ['Settler', 1, 3]],
        cityNames: ['Baltimore', 'Richmond', 'Pittsburgh', 'Savannah', 'Hartford', 'Albany'] },
      { name: 'Great Britain', adjective: 'British', leader: 'George III', portrait: 27, aggression: 0.7, gold: 320, government: 'Monarchy', techs: ENLIGHTENMENT,
        cities: [['London', -0.1, 51.5, 8], ['Bristol', -2.6, 51.5, 4], ['Edinburgh', -3.2, 55.9, 4], ['Dublin', -6.3, 53.3, 4], ['Halifax', -63.6, 44.6, 3], ['Quebec', -71.2, 46.8, 3], ['Kingston', -76.8, 18.0, 2]],
        units: [['Musketman', 3, 0], ['Cannon', 2, 0], ['Frigate', 4, 0], ['Galleon', 3, 1], ['Musketman', 4, 4], ['Cannon', 2, 4], ['Cavalry', 1, 4], ['Frigate', 1, 4], ['Musketman', 3, 5], ['Musketman', 1, 6], ['Worker', 1, 0]],
        cityNames: ['Manchester', 'Plymouth', 'Glasgow', 'Liverpool', 'Montreal', 'Nassau'] },
      { name: 'France', adjective: 'French', leader: 'Louis', portrait: 14, aggression: 0.5, gold: 90, government: 'Monarchy', techs: ENLIGHTENMENT,
        cities: [['Paris', 2.3, 48.9, 6], ['Brest', -4.5, 48.4, 3], ['Bordeaux', -0.6, 44.8, 3], ['Port-au-Prince', -72.3, 18.5, 2]],
        units: [['Musketman', 3, 0], ['Cannon', 1, 0], ['Cavalry', 1, 0], ['Frigate', 2, 1], ['Galleon', 1, 1], ['Musketman', 1, 3], ['Worker', 1, 0]],
        cityNames: ['Marseille', 'Toulouse', 'Nantes', 'Rouen', 'Martinique', 'Cayenne'] },
      { name: 'Spain', adjective: 'Spanish', leader: 'Charles', portrait: 28, aggression: 0.45, gold: 120, government: 'Monarchy', techs: ENLIGHTENMENT,
        cities: [['Madrid', -3.7, 40.4, 7], ['Cadiz', -6.3, 36.5, 4], ['Havana', -82.4, 23.1, 4], ['New Orleans', -90.1, 30.0, 2], ['Veracruz', -96.1, 19.2, 3], ['St. Augustine', -81.3, 29.9, 2]],
        units: [['Musketman', 2, 0], ['Cavalry', 1, 0], ['Frigate', 1, 1], ['Galleon', 1, 1], ['Musketman', 2, 2], ['Frigate', 1, 2], ['Musketman', 1, 3], ['Musketman', 1, 4], ['Worker', 1, 0]],
        cityNames: ['Seville', 'Barcelona', 'San Juan', 'Santo Domingo', 'Pensacola', 'Campeche'] },
      { name: 'the Iroquois', adjective: 'Iroquois', leader: 'Joseph Brant', portrait: 9, aggression: 0.55, gold: 20, government: 'Monarchy', techs: FIRST_NATIONS,
        cities: [['Onondaga', -76.1, 43.0, 3], ['Seneca Castle', -77.6, 42.7, 2]],
        units: [['Horseman', 2, 0], ['Archer', 2, 0], ['Spearman', 1, 1]],
        cityNames: ['Oneida', 'Cayuga', 'Tuscarora', 'Niagara', 'Oswego'] },
    ],
    wars: [['the United States', 'Great Britain'], ['France', 'Great Britain'], ['the United States', 'the Iroquois']],
    alliances: [['the United States', 'France'], ['Great Britain', 'the Iroquois']],
  },
  {
    id: 'napoleon', expansion: 'revolution',
    name: 'The Napoleonic Wars',
    blurb: '1805. Napoleon has crowned himself Emperor. Britain, Austria and Russia stand together in the Third Coalition; Spain marches with France, Prussia and the Ottomans wait.',
    startYear: 1805, yearsPerTurn: 1, known: true, map: EUROPE,
    civs: [
      { name: 'France', adjective: 'French', leader: 'Napoleon', portrait: 14, aggression: 0.7, gold: 120, government: 'Monarchy', techs: REVOLUTION,
        cities: [['Paris', 2.3, 48.9, 8], ['Lyon', 4.8, 45.8, 4], ['Marseille', 5.4, 43.3, 3], ['Bordeaux', -0.6, 44.8, 4], ['Brest', -4.5, 48.4, 3], ['Brussels', 4.35, 50.85, 4], ['Amsterdam', 4.9, 52.4, 4], ['Milan', 9.2, 45.5, 4]],
        units: [['Musketman', 3, 0], ['Cannon', 2, 0], ['Cavalry', 2, 0], ['Musketman', 2, 5], ['Cavalry', 1, 5], ['Musketman', 2, 7], ['Cannon', 1, 7], ['Frigate', 2, 4], ['Musketman', 1, 6], ['Worker', 2, 0]],
        cityNames: ['Toulouse', 'Nantes', 'Strasbourg', 'Turin', 'Genoa', 'Antwerp'] },
      { name: 'Great Britain', adjective: 'British', leader: 'George III', portrait: 27, aggression: 0.55, gold: 300, government: 'Monarchy', techs: REVOLUTION,
        cities: [['London', -0.1, 51.5, 9], ['Edinburgh', -3.2, 55.9, 4], ['Dublin', -6.3, 53.3, 4], ['Plymouth', -4.1, 50.4, 3], ['Gibraltar', -5.35, 36.14, 1], ['Valletta', 14.5, 35.9, 1]],
        units: [['Musketman', 3, 0], ['Cannon', 1, 0], ['Frigate', 3, 3], ['Galleon', 2, 3], ['Frigate', 1, 4], ['Musketman', 1, 4], ['Frigate', 1, 5], ['Musketman', 1, 5], ['Worker', 1, 0]],
        cityNames: ['Manchester', 'Bristol', 'Glasgow', 'Liverpool', 'Portsmouth', 'Cork'] },
      { name: 'Austria', adjective: 'Austrian', leader: 'Francis', portrait: 24, aggression: 0.5, gold: 150, government: 'Monarchy', techs: ENLIGHTENMENT,
        cities: [['Vienna', 16.4, 48.2, 8], ['Prague', 14.4, 50.1, 5], ['Budapest', 19.0, 47.5, 5], ['Venice', 12.3, 45.4, 4], ['Lemberg', 24.0, 49.8, 3]],
        units: [['Musketman', 4, 0], ['Cannon', 2, 0], ['Cavalry', 3, 0], ['Musketman', 3, 3], ['Musketman', 1, 1], ['Cavalry', 1, 2], ['Cannon', 1, 3], ['Worker', 1, 0]],
        cityNames: ['Graz', 'Innsbruck', 'Salzburg', 'Trieste', 'Linz', 'Brno'] },
      { name: 'Prussia', adjective: 'Prussian', leader: 'Frederick William', portrait: 23, aggression: 0.55, gold: 150, government: 'Monarchy', techs: ENLIGHTENMENT,
        cities: [['Berlin', 13.4, 52.5, 7], ['Königsberg', 20.5, 54.7, 4], ['Breslau', 17.0, 51.1, 4], ['Magdeburg', 11.6, 52.1, 3], ['Danzig', 18.6, 54.35, 3]],
        units: [['Musketman', 4, 0], ['Cavalry', 3, 0], ['Cannon', 2, 0], ['Musketman', 1, 1], ['Musketman', 1, 2], ['Musketman', 1, 3], ['Worker', 1, 0]],
        cityNames: ['Potsdam', 'Danzig', 'Stettin', 'Halle', 'Posen', 'Münster'] },
      { name: 'Russia', adjective: 'Russian', leader: 'Alexander', portrait: 16, aggression: 0.6, gold: 120, government: 'Monarchy', techs: ENLIGHTENMENT,
        cities: [['Saint Petersburg', 30.3, 59.9, 7], ['Moscow', 37.6, 55.8, 8], ['Vilna', 25.3, 54.7, 3], ['Kiev', 30.5, 50.45, 4], ['Riga', 24.1, 56.9, 3]],
        units: [['Musketman', 3, 1], ['Cavalry', 3, 1], ['Musketman', 4, 2], ['Cannon', 2, 2], ['Cavalry', 2, 3], ['Musketman', 1, 0], ['Frigate', 1, 0], ['Worker', 1, 1]],
        cityNames: ['Smolensk', 'Minsk', 'Odessa', 'Novgorod', 'Tula', 'Kharkov'] },
      { name: 'Spain', adjective: 'Spanish', leader: 'Charles', portrait: 28, aggression: 0.4, gold: 140, government: 'Monarchy', techs: ENLIGHTENMENT,
        cities: [['Madrid', -3.7, 40.4, 7], ['Seville', -6.0, 37.4, 4], ['Barcelona', 2.2, 41.4, 4], ['Cadiz', -6.3, 36.5, 3]],
        units: [['Musketman', 2, 0], ['Cavalry', 1, 0], ['Frigate', 2, 3], ['Musketman', 1, 2], ['Worker', 1, 0]],
        cityNames: ['Valencia', 'Zaragoza', 'Granada', 'Bilbao', 'Toledo', 'Lisbon'] },
      { name: 'the Ottomans', adjective: 'Ottoman', leader: 'Selim', portrait: 15, aggression: 0.5, gold: 160, government: 'Monarchy', techs: ENLIGHTENMENT,
        cities: [['Constantinople', 29.0, 41.0, 8], ['Edirne', 26.6, 41.7, 4], ['Salonica', 22.9, 40.6, 4], ['Smyrna', 27.1, 38.4, 4], ['Ankara', 32.9, 39.9, 3]],
        units: [['Musketman', 3, 0], ['Cavalry', 2, 0], ['Cannon', 1, 0], ['Musketman', 1, 2], ['Frigate', 1, 0], ['Worker', 1, 0]],
        cityNames: ['Bursa', 'Sofia', 'Belgrade', 'Athens', 'Trebizond', 'Konya'] },
    ],
    wars: [['France', 'Great Britain'], ['France', 'Austria'], ['France', 'Russia'], ['Spain', 'Great Britain']],
    alliances: [['Great Britain', 'Austria'], ['Great Britain', 'Russia'], ['Austria', 'Russia'], ['France', 'Spain']],
  },
  {
    id: 'greatwar', expansion: 'worldwars',
    name: 'The Great War',
    blurb: '1914. The Central Powers - Germany and Austria-Hungary - face France, Russia and Britain across Europe. Italy is neutral; the Ottomans lean towards Berlin.',
    startYear: 1914, yearsPerTurn: 1, known: true, map: GREAT_WAR_MAP,
    civs: [
      { name: 'Germany', adjective: 'German', leader: 'Wilhelm', portrait: 25, aggression: 0.8, gold: 400, government: 'Monarchy', techs: GREAT_WAR,
        cities: [['Berlin', 13.4, 52.5, 10], ['Hamburg', 10.0, 53.55, 7], ['Munich', 11.6, 48.1, 6], ['Cologne', 6.95, 50.94, 7], ['Königsberg', 20.5, 54.7, 4], ['Breslau', 17.0, 51.1, 5], ['Strasbourg', 7.75, 48.6, 4]],
        units: [['Rifleman', 5, 3], ['Artillery', 3, 3], ['Cavalry', 2, 3], ['Rifleman', 4, 6], ['Artillery', 2, 6], ['Rifleman', 3, 4], ['Cavalry', 1, 4], ['Rifleman', 2, 0], ['Battleship', 2, 1], ['Destroyer', 1, 1], ['Engineers', 1, 0]],
        cityNames: ['Frankfurt', 'Leipzig', 'Dresden', 'Stuttgart', 'Bremen', 'Danzig'] },
      { name: 'Austria-Hungary', adjective: 'Austro-Hungarian', leader: 'Franz Joseph', portrait: 24, aggression: 0.6, gold: 200, government: 'Monarchy', techs: GREAT_WAR,
        cities: [['Vienna', 16.4, 48.2, 9], ['Budapest', 19.0, 47.5, 7], ['Prague', 14.4, 50.1, 6], ['Trieste', 13.8, 45.65, 4], ['Sarajevo', 18.4, 43.9, 3], ['Lemberg', 24.0, 49.8, 4]],
        units: [['Rifleman', 3, 5], ['Artillery', 1, 5], ['Rifleman', 3, 4], ['Rifleman', 2, 0], ['Artillery', 1, 0], ['Cavalry', 1, 1], ['Battleship', 1, 3], ['Engineers', 1, 0]],
        cityNames: ['Graz', 'Krakow', 'Zagreb', 'Innsbruck', 'Brno', 'Pressburg'] },
      { name: 'France', adjective: 'French', leader: 'Poincaré', portrait: 14, aggression: 0.55, gold: 260, government: 'Democracy', techs: GREAT_WAR,
        cities: [['Paris', 2.3, 48.9, 10], ['Lyon', 4.8, 45.8, 6], ['Marseille', 5.4, 43.3, 6], ['Bordeaux', -0.6, 44.8, 5], ['Lille', 3.06, 50.6, 5], ['Nancy', 6.2, 48.7, 4], ['Algiers', 3.06, 36.75, 4]],
        units: [['Rifleman', 4, 5], ['Artillery', 2, 5], ['Rifleman', 3, 4], ['Cavalry', 1, 4], ['Rifleman', 2, 0], ['Artillery', 1, 0], ['Battleship', 1, 2], ['Destroyer', 1, 2], ['Engineers', 1, 0]],
        cityNames: ['Toulouse', 'Nantes', 'Rouen', 'Brest', 'Verdun', 'Tunis'] },
      { name: 'the United Kingdom', adjective: 'British', leader: 'George V', portrait: 27, aggression: 0.5, gold: 400, government: 'Democracy', techs: GREAT_WAR,
        cities: [['London', -0.1, 51.5, 11], ['Manchester', -2.24, 53.5, 7], ['Glasgow', -4.25, 55.86, 6], ['Dublin', -6.3, 53.3, 5], ['Gibraltar', -5.35, 36.14, 1], ['Valletta', 14.5, 35.9, 1], ['Cairo', 31.2, 30.0, 5]],
        units: [['Rifleman', 3, 0], ['Artillery', 1, 0], ['Transport', 2, 0], ['Battleship', 3, 0], ['Destroyer', 2, 0], ['Rifleman', 1, 4], ['Battleship', 1, 5], ['Rifleman', 2, 6], ['Engineers', 1, 0]],
        cityNames: ['Birmingham', 'Liverpool', 'Edinburgh', 'Belfast', 'Plymouth', 'Alexandria'] },
      { name: 'Russia', adjective: 'Russian', leader: 'Nicholas', portrait: 16, aggression: 0.6, gold: 200, government: 'Monarchy', techs: GREAT_WAR.filter((t) => t !== 'Steel'),
        cities: [['Saint Petersburg', 30.3, 59.9, 9], ['Moscow', 37.6, 55.8, 9], ['Warsaw', 21.0, 52.2, 6], ['Kiev', 30.5, 50.45, 6], ['Odessa', 30.7, 46.5, 5], ['Riga', 24.1, 56.9, 4]],
        units: [['Rifleman', 4, 2], ['Cavalry', 2, 2], ['Artillery', 1, 2], ['Rifleman', 3, 3], ['Cavalry', 1, 3], ['Rifleman', 2, 5], ['Rifleman', 2, 1], ['Ironclad', 1, 4], ['Engineers', 1, 1]],
        cityNames: ['Minsk', 'Smolensk', 'Kharkov', 'Tula', 'Kazan', 'Sevastopol'] },
      { name: 'Italy', adjective: 'Italian', leader: 'Victor Emmanuel', portrait: 26, aggression: 0.45, gold: 180, government: 'Monarchy', techs: GREAT_WAR,
        cities: [['Rome', 12.5, 41.9, 8], ['Milan', 9.2, 45.5, 7], ['Naples', 14.25, 40.85, 6], ['Turin', 7.7, 45.07, 5], ['Venice', 12.3, 45.44, 4], ['Palermo', 13.36, 38.1, 4], ['Tripoli', 13.2, 32.9, 2]],
        units: [['Rifleman', 3, 1], ['Artillery', 1, 1], ['Rifleman', 2, 4], ['Rifleman', 2, 0], ['Battleship', 1, 2], ['Destroyer', 1, 2], ['Rifleman', 1, 6], ['Engineers', 1, 0]],
        cityNames: ['Florence', 'Genoa', 'Bologna', 'Bari', 'Trento', 'Cagliari'] },
      { name: 'the Ottomans', adjective: 'Ottoman', leader: 'Mehmed', portrait: 15, aggression: 0.55, gold: 120, government: 'Monarchy', techs: GREAT_WAR.filter((t) => t !== 'Steel' && t !== 'Refining'),
        cities: [['Constantinople', 29.0, 41.0, 8], ['Ankara', 32.9, 39.9, 4], ['Smyrna', 27.1, 38.4, 5], ['Damascus', 36.3, 33.5, 5], ['Baghdad', 44.4, 33.3, 5], ['Trebizond', 39.7, 41.0, 3]],
        units: [['Rifleman', 3, 0], ['Artillery', 1, 0], ['Rifleman', 2, 5], ['Rifleman', 2, 3], ['Cavalry', 1, 3], ['Rifleman', 1, 4], ['Ironclad', 1, 0], ['Engineers', 1, 0]],
        cityNames: ['Edirne', 'Bursa', 'Konya', 'Aleppo', 'Mosul', 'Jerusalem'] },
    ],
    wars: [['Germany', 'France'], ['Germany', 'Russia'], ['Germany', 'the United Kingdom'], ['Austria-Hungary', 'Russia'], ['Austria-Hungary', 'France'], ['Austria-Hungary', 'the United Kingdom']],
    alliances: [['Germany', 'Austria-Hungary'], ['Germany', 'the Ottomans'], ['France', 'Russia'], ['France', 'the United Kingdom'], ['the United Kingdom', 'Russia']],
  },
  {
    id: 'ww2', expansion: 'worldwars',
    name: 'The Second World War',
    blurb: '1939. Germany has invaded its neighbour, and Britain, her Commonwealth and France have declared war; Japan is at war in China. Italy and Japan are Germany\'s allies. The United States, the Soviet Union and Latin America are, for now, at peace.',
    startYear: 1939, yearsPerTurn: 1, known: true, map: WORLD,
    civs: [
      { name: 'Germany', adjective: 'German', leader: 'The Chancellor', portrait: 18, aggression: 0.85, gold: 550, government: 'Fundamentalism', techs: WORLD_WAR,
        cities: [['Berlin', 13.4, 52.5, 12], ['Munich', 11.6, 48.1, 8], ['Hamburg', 10.0, 53.55, 8], ['Vienna', 16.4, 48.2, 8], ['Königsberg', 20.5, 54.7, 5], ['Cologne', 6.95, 50.94, 8]],
        units: [['Tank', 3, 0], ['Infantry', 3, 0], ['Artillery', 2, 0], ['Fighter', 2, 0], ['Bomber', 2, 0], ['Infantry', 2, 4], ['Tank', 2, 4], ['Infantry', 2, 1], ['Tank', 2, 5], ['Infantry', 2, 5], ['Artillery', 1, 5], ['Submarine', 3, 2], ['Battleship', 1, 2], ['Engineers', 1, 0]],
        cityNames: ['Cologne', 'Frankfurt', 'Leipzig', 'Dresden', 'Stuttgart', 'Bremen'] },
      { name: 'Italy', adjective: 'Italian', leader: 'The Prime Minister', portrait: 20, aggression: 0.6, gold: 240, government: 'Fundamentalism', techs: WORLD_WAR,
        cities: [['Rome', 12.5, 41.9, 9], ['Milan', 9.2, 45.5, 8], ['Naples', 14.25, 40.85, 6], ['Tripoli', 13.2, 32.9, 3], ['Addis Ababa', 38.7, 9.0, 3]],
        units: [['Infantry', 3, 0], ['Artillery', 1, 0], ['Tank', 1, 1], ['Infantry', 2, 1], ['Battleship', 2, 2], ['Destroyer', 1, 2], ['Infantry', 2, 3], ['Infantry', 1, 4], ['Fighter', 1, 0], ['Engineers', 1, 0]],
        cityNames: ['Turin', 'Florence', 'Genoa', 'Venice', 'Palermo', 'Bologna'] },
      { name: 'Japan', adjective: 'Japanese', leader: 'The Prime Minister', portrait: 18, aggression: 0.8, gold: 300, government: 'Monarchy', techs: WORLD_WAR,
        cities: [['Tokyo', 139.7, 35.7, 11], ['Osaka', 135.5, 34.7, 8], ['Seoul', 127.0, 37.5, 6], ['Mukden', 123.4, 41.8, 5], ['Taipei', 121.5, 25.0, 4]],
        units: [['Infantry', 3, 0], ['Carrier', 2, 0], ['Fighter', 3, 0], ['Battleship', 2, 1], ['Destroyer', 2, 1], ['Transport', 2, 1], ['Infantry', 3, 3], ['Tank', 1, 3], ['Artillery', 1, 3], ['Infantry', 2, 2], ['Infantry', 1, 4], ['Engineers', 1, 0]],
        cityNames: ['Nagoya', 'Kyoto', 'Hiroshima', 'Sapporo', 'Fukuoka', 'Sendai'] },
      { name: 'the United Kingdom', adjective: 'British', leader: 'The Prime Minister', portrait: 20, aggression: 0.5, gold: 500, government: 'Democracy', techs: WORLD_WAR,
        cities: [['London', -0.1, 51.5, 12], ['Glasgow', -4.25, 55.86, 7], ['Cairo', 31.2, 30.0, 6], ['Delhi', 77.2, 28.6, 8], ['Bombay', 72.9, 19.1, 7], ['Singapore', 103.8, 1.35, 4], ['Hong Kong', 114.2, 22.3, 3], ['Ottawa', -75.7, 45.4, 6], ['Vancouver', -123.1, 49.3, 4], ['Sydney', 151.2, -33.9, 6], ['Perth', 115.9, -31.95, 3], ['Cape Town', 18.4, -33.9, 4], ['Johannesburg', 28.0, -26.2, 4]],
        units: [['Infantry', 3, 0], ['Fighter', 3, 0], ['Bomber', 1, 0], ['Battleship', 2, 1], ['Destroyer', 2, 1], ['Carrier', 1, 1], ['Transport', 2, 0], ['Infantry', 2, 2], ['Tank', 1, 2], ['Infantry', 2, 3], ['Infantry', 1, 4], ['Infantry', 1, 5], ['Infantry', 1, 6], ['Engineers', 1, 0]],
        cityNames: ['Manchester', 'Birmingham', 'Liverpool', 'Calcutta', 'Madras', 'Rangoon'] },
      { name: 'France', adjective: 'French', leader: 'The President', portrait: 21, aggression: 0.45, gold: 350, government: 'Democracy', techs: WORLD_WAR,
        cities: [['Paris', 2.3, 48.9, 11], ['Marseille', 5.4, 43.3, 7], ['Algiers', 3.06, 36.75, 5], ['Dakar', -17.4, 14.7, 3], ['Hanoi', 105.8, 21.0, 4]],
        units: [['Infantry', 4, 0], ['Tank', 2, 0], ['Artillery', 2, 0], ['Fighter', 1, 0], ['Battleship', 1, 1], ['Destroyer', 1, 1], ['Infantry', 2, 2], ['Infantry', 1, 3], ['Infantry', 1, 4], ['Engineers', 1, 0]],
        cityNames: ['Lyon', 'Bordeaux', 'Lille', 'Toulouse', 'Tunis', 'Saigon'] },
      { name: 'the Soviet Union', adjective: 'Soviet', leader: 'The General Secretary', portrait: 19, aggression: 0.6, gold: 300, government: 'Communism', techs: [...WORLD_WAR.filter((t) => t !== 'Advanced Flight' && t !== 'Democracy'), 'Communism'],
        cities: [['Moscow', 37.6, 55.8, 12], ['Leningrad', 30.3, 59.9, 9], ['Kiev', 30.5, 50.45, 8], ['Stalingrad', 44.5, 48.7, 6], ['Novosibirsk', 82.9, 55.0, 4], ['Vladivostok', 131.9, 43.1, 3]],
        units: [['Infantry', 4, 0], ['Tank', 2, 0], ['Artillery', 2, 0], ['Fighter', 1, 0], ['Infantry', 3, 2], ['Tank', 1, 2], ['Infantry', 2, 1], ['Infantry', 1, 3], ['Infantry', 1, 4], ['Infantry', 2, 5], ['Engineers', 1, 0]],
        cityNames: ['Kharkov', 'Gorky', 'Sverdlovsk', 'Kazan', 'Tashkent', 'Murmansk'] },
      { name: 'the United States', adjective: 'American', leader: 'The President', portrait: 18, aggression: 0.35, gold: 350, government: 'Democracy', techs: WORLD_WAR,
        cities: [['Washington', -77.0, 38.9, 10], ['New York', -74.0, 40.7, 12], ['Chicago', -87.6, 41.9, 9], ['Los Angeles', -118.2, 34.0, 7], ['San Francisco', -122.4, 37.8, 6], ['Honolulu', -157.8, 21.3, 2], ['Manila', 121.0, 14.6, 4]],
        units: [['Infantry', 2, 0], ['Fighter', 1, 0], ['Battleship', 1, 1], ['Destroyer', 1, 1], ['Battleship', 2, 5], ['Carrier', 1, 5], ['Fighter', 1, 5], ['Infantry', 1, 4], ['Infantry', 1, 6], ['Engineers', 1, 0]],
        cityNames: ['Philadelphia', 'Boston', 'Detroit', 'Houston', 'Seattle', 'Atlanta'] },
      { name: 'China', adjective: 'Chinese', leader: 'The President', portrait: 21, aggression: 0.45, gold: 150, government: 'Republic', techs: GREAT_WAR,
        cities: [['Chongqing', 106.5, 29.6, 8], ["Xi'an", 108.9, 34.3, 6], ['Kunming', 102.7, 25.0, 5], ['Chengdu', 104.0, 30.7, 6], ['Changsha', 112.9, 28.2, 5]],
        units: [['Rifleman', 4, 0], ['Rifleman', 3, 4], ['Artillery', 1, 4], ['Rifleman', 2, 1], ['Rifleman', 1, 2], ['Rifleman', 1, 3], ['Engineers', 1, 0]],
        cityNames: ['Guiyang', 'Lanzhou', 'Guilin', 'Yan\'an', 'Lhasa', 'Urumqi'] },
      { name: 'Brazil', adjective: 'Brazilian', leader: 'The President', portrait: 19, aggression: 0.3, gold: 200, government: 'Republic', techs: GREAT_WAR,
        cities: [['Rio de Janeiro', -43.2, -22.9, 7], ['São Paulo', -46.6, -23.5, 6], ['Salvador', -38.5, -12.97, 4], ['Manaus', -60.0, -3.1, 2]],
        units: [['Rifleman', 2, 0], ['Rifleman', 1, 1], ['Rifleman', 1, 2], ['Destroyer', 1, 0], ['Engineers', 1, 0]],
        cityNames: ['Recife', 'Belém', 'Porto Alegre', 'Fortaleza', 'Curitiba', 'Natal'] },
      { name: 'Argentina', adjective: 'Argentine', leader: 'The President', portrait: 20, aggression: 0.3, gold: 150, government: 'Republic', techs: GREAT_WAR,
        cities: [['Buenos Aires', -58.4, -34.6, 7], ['Córdoba', -64.2, -31.4, 4], ['Santiago', -70.6, -33.4, 4], ['Lima', -77.0, -12.0, 4]],
        units: [['Rifleman', 2, 0], ['Rifleman', 1, 1], ['Rifleman', 1, 2], ['Rifleman', 1, 3], ['Engineers', 1, 0]],
        cityNames: ['Rosario', 'Mendoza', 'Montevideo', 'Valparaíso', 'La Paz', 'Asunción'] },
      { name: 'Mexico', adjective: 'Mexican', leader: 'The President', portrait: 21, aggression: 0.3, gold: 150, government: 'Republic', techs: GREAT_WAR,
        cities: [['Mexico City', -99.1, 19.4, 7], ['Guadalajara', -103.3, 20.7, 4], ['Monterrey', -100.3, 25.7, 3]],
        units: [['Rifleman', 2, 0], ['Rifleman', 1, 1], ['Rifleman', 1, 2], ['Engineers', 1, 0]],
        cityNames: ['Veracruz', 'Puebla', 'Havana', 'Caracas', 'Quito', 'Panama'] },
    ],
    wars: [['Germany', 'the United Kingdom'], ['Germany', 'France'], ['Japan', 'China']],
    alliances: [['Germany', 'Italy'], ['Germany', 'Japan'], ['Italy', 'Japan'], ['the United Kingdom', 'France']],
  },
];

module.exports = { packs, scenarios, EARTH, knowing };
