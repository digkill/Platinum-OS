import QtQuick

// Логотип: три пересекающихся стеклянных кольца.
//
// Рисуется Canvas, а не растровой картинкой: логотип нужен в разных размерах
// (заставка, иконка, «О системе»), и векторная отрисовка не мылит края.
Item {
    id: logo

    property color ringTop: "#8a7ce8"
    property color ringLeft: "#a78bea"
    property color ringRight: "#7ab6ee"
    property real thickness: Math.max(2, width * 0.075)

    Canvas {
        anchors.fill: parent
        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();

            const r = width * 0.215;
            const cx = width / 2;
            const cy = height / 2;
            // Кольца ставятся треугольником: верхнее и два нижних внахлёст.
            const rings = [
                { x: cx,             y: cy - r * 0.72, c: logo.ringTop },
                { x: cx - r * 0.92,  y: cy + r * 0.68, c: logo.ringLeft },
                { x: cx + r * 0.92,  y: cy + r * 0.68, c: logo.ringRight }
            ];

            ctx.lineWidth = logo.thickness;
            for (const ring of rings) {
                // Полупрозрачность даёт эффект стекла в местах пересечения.
                ctx.globalAlpha = 0.78;
                ctx.strokeStyle = ring.c;
                ctx.beginPath();
                ctx.arc(ring.x, ring.y, r, 0, Math.PI * 2);
                ctx.stroke();

                // Блик по верхней дуге.
                ctx.globalAlpha = 0.5;
                ctx.strokeStyle = "#ffffff";
                ctx.lineWidth = logo.thickness * 0.32;
                ctx.beginPath();
                ctx.arc(ring.x, ring.y, r, Math.PI * 1.15, Math.PI * 1.75);
                ctx.stroke();
                ctx.lineWidth = logo.thickness;
            }
        }
    }
}
