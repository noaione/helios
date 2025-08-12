(() => {
    // interface HostData {
    //     host: string;
    //     lines: {
    //         key: string;
    //         value: string;
    //     }[];
    // }

    const CLEAR_SPEED = 75; // ms
    const WRITE_SPEED = 50; // ms
    const START_DELAY = 500; // ms

    // Global abort controller for fetch requests so we don't have multiple fetches running at the same time
    const state = {
        controller: new AbortController(),
    };

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
            const response = await fetch('/s', {
                signal: state.controller.signal, // use the global abort controller
            });

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
        setInterval(() => {
            // abort any ongoing fetch request
            if (state.controller.signal.aborted) {
                state.controller = new AbortController(); // reset the controller if it was aborted
            } else {
                state.controller.abort(); // abort the ongoing request
                state.controller = new AbortController(); // create a new controller for the next request
            }

            refreshData();
        }, 20000); // refresh every 20 seconds
        refreshData(); // initial fetch

        /**
         * Write text to the console.
         * @param {string} text input text to write to the console
         */
        window.toConsole = (text) => {
            if (typeof text !== 'string') {
                console.error('Input text must be a string');
                return;
            }

            // sanitize the input text
            const spanElement = document.createElement('span');
            spanElement.textContent = text; // this will escape any HTML tags
            const realText = spanElement.textContent;

            if (realText.trim() === '') {
                clearWritingArea();
            } else {
                animateWritingArea(realText.trim());
            }
        }

        window.clearConsole = () => {
            clearWritingArea();
        }

        const cursor = document.querySelector('[data-id="cursor"]');
        if (cursor) {
            // when clicked, alert the current text in the writing area
            cursor.addEventListener('click', () => {
                const toBeWritten = prompt('Enter text to write to the console, leave empty to clear it out:', '');
                if (toBeWritten !== null) {
                    if (toBeWritten.trim() === '') {
                        clearWritingArea();
                    } else {
                        animateWritingArea(toBeWritten.trim());
                    }
                }
            })
        }
    }

    /**
     * Clear the writing area with an animation effect.
     * @param {HTMLSpanElement | null} element the element to clear, if null, it will use the writing area
     * @returns {void}
     */
    function clearWritingArea(element = null) {
        /** @type {HTMLSpanElement} */
        const writingArea = element ?? document.querySelector('[data-id="writing-area"]');
        if (!writingArea) {
            console.error('Writing area not found');
            return;
        }

        const currentText = writingArea.textContent.trim();
        if (currentText.length > 0) {
            // animate clearing the text, clear one char every 100ms
            let index = currentText.length - 1;
            const cleanId = setInterval(() => {
                if (index < 0) {
                    clearInterval(cleanId);
                    writingArea.textContent = ''; // clear the text
                    return;
                }
                writingArea.textContent = currentText.slice(0, index);
                index--;
            }, CLEAR_SPEED);
        }

        return currentText.length; // return true if there was text to clear
    }

    /**
     * Write text to the writing area with an animation effect.
     * @param {string} inputText 
     * @returns {void}
     */
    function animateWritingArea(inputText) {
        // check if any text is present
        /** @type {HTMLSpanElement} */
        const writingArea = document.querySelector('[data-id="writing-area"]');
        if (!writingArea) {
            console.error('Writing area not found');
            return;
        }

        const textClearAmount = clearWritingArea(writingArea); // clear the writing area first

        /**
         * Write text to the writing area with a real animation effect.
         * @param {string} text the text to write
         */
        const realWritingAnimation = (text) => {
            writingArea.textContent = ''; // clear the text
            let writeIndex = 0;

            const writeId = setInterval(() => {
                if (writeIndex >= text.length) {
                    clearInterval(writeId);
                    return;
                }
                writingArea.textContent += text[writeIndex];
                writeIndex++;
            }, WRITE_SPEED); // write one char every 75ms
        }

        // wait until the text is cleared, then write the new text
        if (textClearAmount > 0) {
            setTimeout(() => {
                realWritingAnimation(inputText);
            }, (textClearAmount * CLEAR_SPEED) + START_DELAY); // wait for the clear animation to finish
        } else {
            // immediately write the text if there was no text to clear
            realWritingAnimation(inputText);
        }
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', start);
    } else {
        start();
    }
})();