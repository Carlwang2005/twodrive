// All coordinates are compositor logical pixels, including negative monitors.
export function panelBounds(icon, area) {
    const [ix, iy, iw, ih] = icon;
    const [ax, ay, aw, ah] = area;
    const width = Math.min(390, Math.max(1, aw - 16));
    const height = Math.min(570, Math.max(1, ah - 16));
    const center = ix + Math.floor(iw / 2);
    const x = Math.max(ax + 8, Math.min(center - Math.floor(width / 2), ax + aw - width - 8));
    const y = iy + Math.floor(ih / 2) < ay ? ay + 8 : ay + ah - height - 8;
    return [x, y, width, height];
}

export function ownIndicator(actor, pid) {
    const legacy = actor?._icon;
    if (legacy?.wm_class === 'twodrive-development' && legacy.pid === pid)
        return legacy;
    const sni = actor?._indicator;
    if (sni?.id === 'twodrive-development' &&
        (sni.busName === `org.kde.StatusNotifierItem-${pid}-1` ||
            sni.uniqueId === `org.kde.StatusNotifierItem-${pid}-1`))
        return actor;
    return null;
}
