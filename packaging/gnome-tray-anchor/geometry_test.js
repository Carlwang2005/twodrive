import {ownIndicator, panelBounds} from './geometry.js';
function equal(actual, expected) {
    if (JSON.stringify(actual) !== JSON.stringify(expected))
        throw Error(`${JSON.stringify(actual)} != ${JSON.stringify(expected)}`);
}
equal(panelBounds([850, 850, 20, 20], [0, 0, 1440, 843]), [665, 265, 390, 570]);
equal(panelBounds([710, 5, 20, 20], [0, 30, 1440, 870]), [525, 38, 390, 570]);
equal(panelBounds([-700, 1050, 20, 20], [-1920, 0, 1920, 1040]), [-885, 462, 390, 570]);
equal(panelBounds([120, 485, 20, 20], [0, 0, 320, 480]), [8, 8, 304, 464]);
const legacy = {_icon: {wm_class: 'twodrive-development', pid: 42}};
equal(ownIndicator(legacy, 42), legacy._icon);
equal(ownIndicator(legacy, 43), null);
equal(ownIndicator({_icon: {wm_class: 'other', pid: 42}}, 42), null);
const sni = {_indicator: {id: 'twodrive-development', busName: 'org.kde.StatusNotifierItem-42-1'}};
equal(ownIndicator(sni, 42), sni);
equal(ownIndicator(sni, 43), null);
print('TwoDrive host geometry/identity checks passed');
