// What only the application object can change, through Qt's own static
// functions: no Rust reference to the application is held meanwhile.
#pragma once
#include <QtCore/QList>
#include <QtCore/QRect>
#include <QtCore/QString>

// The icon every window carries, given to the desktop with the window.
void setWindowIcon(const QString &resource);

// New texts take this font family; an empty one gives the system's font.
// Returns the family in use.
QString setFontFamily(const QString &family);

// The part of each screen a window may cover, in logical pixels, the primary screen first.
QList<QRect> screenAreas();
