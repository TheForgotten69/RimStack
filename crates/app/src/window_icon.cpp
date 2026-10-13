#include "window_icon.h"

#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>

void setWindowIcon(const QString &resource)
{
    QGuiApplication::setWindowIcon(QIcon(resource));
}
