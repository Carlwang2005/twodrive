#ifndef TWODRIVE_TRAY_GEOMETRY_H_
#define TWODRIVE_TRAY_GEOMETRY_H_

#include <algorithm>
#include <cstdlib>

namespace twodrive {
struct Rectangle {
  int x, y, width, height;
};
struct TrayAnchor {
  int x = 0, y = 0;
  Rectangle area = {};
  bool valid = false;

  void capture(int next_x, int next_y, Rectangle next_area) {
    // A host supplies a click point, not the icon's bounds. Preserve the first
    // point for subsequent clicks inside that icon so the panel does not jitter.
    if (valid && area.x == next_area.x && area.y == next_area.y &&
        area.width == next_area.width && area.height == next_area.height &&
        std::abs(x - next_x) <= 28 && std::abs(y - next_y) <= 28) return;
    x = next_x;
    y = next_y;
    area = next_area;
    valid = true;
  }

  Rectangle panel() const {
    const int width = std::min(390, std::max(1, area.width - 16));
    const int height = std::min(570, std::max(1, area.height - 16));
    const int left = std::max(area.x + 8,
        std::min(x + 20 - width, area.x + area.width - width - 8));
    // Prefer above the icon; a top-panel icon opens below it instead.
    const int preferred_y = y - height - 20 >= area.y + 8
        ? y - height - 20 : y + 20;
    const int top = std::max(area.y + 8,
        std::min(preferred_y, area.y + area.height - height - 8));
    return {left, top, width, height};
  }
};
}  // namespace twodrive
#endif
