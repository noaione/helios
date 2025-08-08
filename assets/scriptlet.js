(() => {
    // interface HostData {
    //     host: string;
    //     lines: {
    //         key: string;
    //         value: string;
    //     }[];
    // }

    function writeDataToHTML(data) {
        // create the host header first
        const base = document.querySelector('#detail');
        const clonedBase = base.cloneNode(true);
        clonedBase.innerHTML = ''; // clear the content

        const hostHeader = document.createElement('p');
        hostHeader.className = 'host-header';
        hostHeader.innerHTML = `noaione<span class="host-at">@</span>${data.host}`;

        clonedBase.appendChild(hostHeader);

        // make dashed line
        const dashedLine = document.createElement('p');
        dashedLine.className = 'detail-line';
        dashedLine.textContent = '-'.repeat(data.host.length + 8); // 8 is for "noaione@"

        clonedBase.appendChild(dashedLine);

        // make each line
        data.lines.forEach((line) => {
            const lineEl = document.createElement('p');
            lineEl.className = 'detail-line';
            lineEl.innerHTML = `<span class="detail-line-root">${line.key}</span>: ${line.value}`;
            clonedBase.appendChild(lineEl);
        });

        // add break and palette grid
        const breakEl = document.createElement('br');
        clonedBase.appendChild(breakEl);
        const paletteGrid = writePaletteGrid();
        clonedBase.appendChild(paletteGrid);

        // replace the old content with the new
        base.parentNode.replaceChild(clonedBase, base);
    }

    function writePaletteGrid()  {
        const baseGrid = document.createElement('div');

        baseGrid.className = 'grid max-w-fit grid-cols-8 grid-rows-2 gap-0';

        Array.from({ length: 16 }, (_, i) => {
            const colorBox = document.createElement('div');
            colorBox.className = `block-palette palette-${i + 1}`;
            baseGrid.appendChild(colorBox);
        });

        return baseGrid;
    }

    async function refreshData() {
        try {
            const response = await fetch('/s');

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const data = await response.json();
            writeDataToHTML(data);
        } catch (error) {
            console.error('Error fetching data:', error);
        }
    }

    function start() {
        setInterval(refreshData, 20000); // refresh every 20 seconds
        refreshData(); // initial fetch
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', start);
    } else {
        start();
    }
})();