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
    // Толстые кольца: на образце они стеклянные, а тонкая обводка
    // выглядит проволочной.
    property real thickness: Math.max(2, width * 0.105)

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
                ctx.globalAlpha = 0.62;
                ctx.strokeStyle = ring.c;
                ctx.beginPath();
                ctx.arc(ring.x, ring.y, r, 0, Math.PI * 2);
                ctx.stroke();

                // Блик по верхней дуге.
                ctx.globalAlpha = 0.72;
                ctx.strokeStyle = "#ffffff";
                ctx.lineWidth = logo.thickness * 0.30;
                ctx.beginPath();
                ctx.arc(ring.x, ring.y, r * 1.012, Math.PI * 1.08, Math.PI * 1.72);
                ctx.stroke();

                // Внутренняя кромка: отделяет стекло от фона там, где кольца
                // накладываются друг на друга.
                ctx.globalAlpha = 0.30;
                ctx.lineWidth = logo.thickness * 0.16;
                ctx.beginPath();
                ctx.arc(ring.x, ring.y, r - logo.thickness * 0.42, 0, Math.PI * 2);
                ctx.stroke();

                ctx.lineWidth = logo.thickness;
            }
        }
    }
}
