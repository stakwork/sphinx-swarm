
(function(l, r) { if (!l || l.getElementById('livereloadscript')) return; r = l.createElement('script'); r.async = 1; r.src = '//' + (self.location.host || 'localhost').split(':')[0] + ':35729/livereload.js?snipver=1'; r.id = 'livereloadscript'; l.getElementsByTagName('head')[0].appendChild(r) })(self.document);
var app = (function () {
    'use strict';

    function noop() { }
    function add_location(element, file, line, column, char) {
        element.__svelte_meta = {
            loc: { file, line, column, char }
        };
    }
    function run(fn) {
        return fn();
    }
    function blank_object() {
        return Object.create(null);
    }
    function run_all(fns) {
        fns.forEach(run);
    }
    function is_function(thing) {
        return typeof thing === 'function';
    }
    function safe_not_equal(a, b) {
        return a != a ? b == b : a !== b || ((a && typeof a === 'object') || typeof a === 'function');
    }
    function is_empty(obj) {
        return Object.keys(obj).length === 0;
    }
    function validate_store(store, name) {
        if (store != null && typeof store.subscribe !== 'function') {
            throw new Error(`'${name}' is not a store with a 'subscribe' method`);
        }
    }
    function subscribe$1(store, ...callbacks) {
        if (store == null) {
            return noop;
        }
        const unsub = store.subscribe(...callbacks);
        return unsub.unsubscribe ? () => unsub.unsubscribe() : unsub;
    }
    function get_store_value(store) {
        let value;
        subscribe$1(store, _ => value = _)();
        return value;
    }
    function component_subscribe(component, store, callback) {
        component.$$.on_destroy.push(subscribe$1(store, callback));
    }
    function append(target, node) {
        target.appendChild(node);
    }
    function insert(target, node, anchor) {
        target.insertBefore(node, anchor || null);
    }
    function detach(node) {
        node.parentNode.removeChild(node);
    }
    function destroy_each(iterations, detaching) {
        for (let i = 0; i < iterations.length; i += 1) {
            if (iterations[i])
                iterations[i].d(detaching);
        }
    }
    function element(name) {
        return document.createElement(name);
    }
    function svg_element(name) {
        return document.createElementNS('http://www.w3.org/2000/svg', name);
    }
    function text(data) {
        return document.createTextNode(data);
    }
    function space() {
        return text(' ');
    }
    function listen(node, event, handler, options) {
        node.addEventListener(event, handler, options);
        return () => node.removeEventListener(event, handler, options);
    }
    function attr(node, attribute, value) {
        if (value == null)
            node.removeAttribute(attribute);
        else if (node.getAttribute(attribute) !== value)
            node.setAttribute(attribute, value);
    }
    function children(element) {
        return Array.from(element.childNodes);
    }
    function set_input_value(input, value) {
        input.value = value == null ? '' : value;
    }
    function custom_event(type, detail, { bubbles = false, cancelable = false } = {}) {
        const e = document.createEvent('CustomEvent');
        e.initCustomEvent(type, bubbles, cancelable, detail);
        return e;
    }

    let current_component;
    function set_current_component(component) {
        current_component = component;
    }
    function get_current_component() {
        if (!current_component)
            throw new Error('Function called outside component initialization');
        return current_component;
    }
    function onMount(fn) {
        get_current_component().$$.on_mount.push(fn);
    }
    // TODO figure out if we still want to support
    // shorthand events, or if we want to implement
    // a real bubbling mechanism
    function bubble(component, event) {
        const callbacks = component.$$.callbacks[event.type];
        if (callbacks) {
            // @ts-ignore
            callbacks.slice().forEach(fn => fn.call(this, event));
        }
    }

    const dirty_components = [];
    const binding_callbacks = [];
    const render_callbacks = [];
    const flush_callbacks = [];
    const resolved_promise = Promise.resolve();
    let update_scheduled = false;
    function schedule_update() {
        if (!update_scheduled) {
            update_scheduled = true;
            resolved_promise.then(flush);
        }
    }
    function add_render_callback(fn) {
        render_callbacks.push(fn);
    }
    // flush() calls callbacks in this order:
    // 1. All beforeUpdate callbacks, in order: parents before children
    // 2. All bind:this callbacks, in reverse order: children before parents.
    // 3. All afterUpdate callbacks, in order: parents before children. EXCEPT
    //    for afterUpdates called during the initial onMount, which are called in
    //    reverse order: children before parents.
    // Since callbacks might update component values, which could trigger another
    // call to flush(), the following steps guard against this:
    // 1. During beforeUpdate, any updated components will be added to the
    //    dirty_components array and will cause a reentrant call to flush(). Because
    //    the flush index is kept outside the function, the reentrant call will pick
    //    up where the earlier call left off and go through all dirty components. The
    //    current_component value is saved and restored so that the reentrant call will
    //    not interfere with the "parent" flush() call.
    // 2. bind:this callbacks cannot trigger new flush() calls.
    // 3. During afterUpdate, any updated components will NOT have their afterUpdate
    //    callback called a second time; the seen_callbacks set, outside the flush()
    //    function, guarantees this behavior.
    const seen_callbacks = new Set();
    let flushidx = 0; // Do *not* move this inside the flush() function
    function flush() {
        const saved_component = current_component;
        do {
            // first, call beforeUpdate functions
            // and update components
            while (flushidx < dirty_components.length) {
                const component = dirty_components[flushidx];
                flushidx++;
                set_current_component(component);
                update(component.$$);
            }
            set_current_component(null);
            dirty_components.length = 0;
            flushidx = 0;
            while (binding_callbacks.length)
                binding_callbacks.pop()();
            // then, once components are updated, call
            // afterUpdate functions. This may cause
            // subsequent updates...
            for (let i = 0; i < render_callbacks.length; i += 1) {
                const callback = render_callbacks[i];
                if (!seen_callbacks.has(callback)) {
                    // ...so guard against infinite loops
                    seen_callbacks.add(callback);
                    callback();
                }
            }
            render_callbacks.length = 0;
        } while (dirty_components.length);
        while (flush_callbacks.length) {
            flush_callbacks.pop()();
        }
        update_scheduled = false;
        seen_callbacks.clear();
        set_current_component(saved_component);
    }
    function update($$) {
        if ($$.fragment !== null) {
            $$.update();
            run_all($$.before_update);
            const dirty = $$.dirty;
            $$.dirty = [-1];
            $$.fragment && $$.fragment.p($$.ctx, dirty);
            $$.after_update.forEach(add_render_callback);
        }
    }
    const outroing = new Set();
    let outros;
    function group_outros() {
        outros = {
            r: 0,
            c: [],
            p: outros // parent group
        };
    }
    function check_outros() {
        if (!outros.r) {
            run_all(outros.c);
        }
        outros = outros.p;
    }
    function transition_in(block, local) {
        if (block && block.i) {
            outroing.delete(block);
            block.i(local);
        }
    }
    function transition_out(block, local, detach, callback) {
        if (block && block.o) {
            if (outroing.has(block))
                return;
            outroing.add(block);
            outros.c.push(() => {
                outroing.delete(block);
                if (callback) {
                    if (detach)
                        block.d(1);
                    callback();
                }
            });
            block.o(local);
        }
        else if (callback) {
            callback();
        }
    }

    const globals = (typeof window !== 'undefined'
        ? window
        : typeof globalThis !== 'undefined'
            ? globalThis
            : global);
    function create_component(block) {
        block && block.c();
    }
    function mount_component(component, target, anchor, customElement) {
        const { fragment, on_mount, on_destroy, after_update } = component.$$;
        fragment && fragment.m(target, anchor);
        if (!customElement) {
            // onMount happens before the initial afterUpdate
            add_render_callback(() => {
                const new_on_destroy = on_mount.map(run).filter(is_function);
                if (on_destroy) {
                    on_destroy.push(...new_on_destroy);
                }
                else {
                    // Edge case - component was destroyed immediately,
                    // most likely as a result of a binding initialising
                    run_all(new_on_destroy);
                }
                component.$$.on_mount = [];
            });
        }
        after_update.forEach(add_render_callback);
    }
    function destroy_component(component, detaching) {
        const $$ = component.$$;
        if ($$.fragment !== null) {
            run_all($$.on_destroy);
            $$.fragment && $$.fragment.d(detaching);
            // TODO null out other refs, including component.$$ (but need to
            // preserve final state?)
            $$.on_destroy = $$.fragment = null;
            $$.ctx = [];
        }
    }
    function make_dirty(component, i) {
        if (component.$$.dirty[0] === -1) {
            dirty_components.push(component);
            schedule_update();
            component.$$.dirty.fill(0);
        }
        component.$$.dirty[(i / 31) | 0] |= (1 << (i % 31));
    }
    function init(component, options, instance, create_fragment, not_equal, props, append_styles, dirty = [-1]) {
        const parent_component = current_component;
        set_current_component(component);
        const $$ = component.$$ = {
            fragment: null,
            ctx: null,
            // state
            props,
            update: noop,
            not_equal,
            bound: blank_object(),
            // lifecycle
            on_mount: [],
            on_destroy: [],
            on_disconnect: [],
            before_update: [],
            after_update: [],
            context: new Map(options.context || (parent_component ? parent_component.$$.context : [])),
            // everything else
            callbacks: blank_object(),
            dirty,
            skip_bound: false,
            root: options.target || parent_component.$$.root
        };
        append_styles && append_styles($$.root);
        let ready = false;
        $$.ctx = instance
            ? instance(component, options.props || {}, (i, ret, ...rest) => {
                const value = rest.length ? rest[0] : ret;
                if ($$.ctx && not_equal($$.ctx[i], $$.ctx[i] = value)) {
                    if (!$$.skip_bound && $$.bound[i])
                        $$.bound[i](value);
                    if (ready)
                        make_dirty(component, i);
                }
                return ret;
            })
            : [];
        $$.update();
        ready = true;
        run_all($$.before_update);
        // `false` as a special case of no DOM component
        $$.fragment = create_fragment ? create_fragment($$.ctx) : false;
        if (options.target) {
            if (options.hydrate) {
                const nodes = children(options.target);
                // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
                $$.fragment && $$.fragment.l(nodes);
                nodes.forEach(detach);
            }
            else {
                // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
                $$.fragment && $$.fragment.c();
            }
            if (options.intro)
                transition_in(component.$$.fragment);
            mount_component(component, options.target, options.anchor, options.customElement);
            flush();
        }
        set_current_component(parent_component);
    }
    /**
     * Base class for Svelte components. Used when dev=false.
     */
    class SvelteComponent {
        $destroy() {
            destroy_component(this, 1);
            this.$destroy = noop;
        }
        $on(type, callback) {
            const callbacks = (this.$$.callbacks[type] || (this.$$.callbacks[type] = []));
            callbacks.push(callback);
            return () => {
                const index = callbacks.indexOf(callback);
                if (index !== -1)
                    callbacks.splice(index, 1);
            };
        }
        $set($$props) {
            if (this.$$set && !is_empty($$props)) {
                this.$$.skip_bound = true;
                this.$$set($$props);
                this.$$.skip_bound = false;
            }
        }
    }

    function dispatch_dev(type, detail) {
        document.dispatchEvent(custom_event(type, Object.assign({ version: '3.50.1' }, detail), { bubbles: true }));
    }
    function append_dev(target, node) {
        dispatch_dev('SvelteDOMInsert', { target, node });
        append(target, node);
    }
    function insert_dev(target, node, anchor) {
        dispatch_dev('SvelteDOMInsert', { target, node, anchor });
        insert(target, node, anchor);
    }
    function detach_dev(node) {
        dispatch_dev('SvelteDOMRemove', { node });
        detach(node);
    }
    function listen_dev(node, event, handler, options, has_prevent_default, has_stop_propagation) {
        const modifiers = options === true ? ['capture'] : options ? Array.from(Object.keys(options)) : [];
        if (has_prevent_default)
            modifiers.push('preventDefault');
        if (has_stop_propagation)
            modifiers.push('stopPropagation');
        dispatch_dev('SvelteDOMAddEventListener', { node, event, handler, modifiers });
        const dispose = listen(node, event, handler, options);
        return () => {
            dispatch_dev('SvelteDOMRemoveEventListener', { node, event, handler, modifiers });
            dispose();
        };
    }
    function attr_dev(node, attribute, value) {
        attr(node, attribute, value);
        if (value == null)
            dispatch_dev('SvelteDOMRemoveAttribute', { node, attribute });
        else
            dispatch_dev('SvelteDOMSetAttribute', { node, attribute, value });
    }
    function set_data_dev(text, data) {
        data = '' + data;
        if (text.wholeText === data)
            return;
        dispatch_dev('SvelteDOMSetData', { node: text, data });
        text.data = data;
    }
    function validate_each_argument(arg) {
        if (typeof arg !== 'string' && !(arg && typeof arg === 'object' && 'length' in arg)) {
            let msg = '{#each} only iterates over array-like objects.';
            if (typeof Symbol === 'function' && arg && Symbol.iterator in arg) {
                msg += ' You can use a spread to convert this iterable into an array.';
            }
            throw new Error(msg);
        }
    }
    function validate_slots(name, slot, keys) {
        for (const slot_key of Object.keys(slot)) {
            if (!~keys.indexOf(slot_key)) {
                console.warn(`<${name}> received an unexpected slot "${slot_key}".`);
            }
        }
    }
    /**
     * Base class for Svelte components with some minor dev-enhancements. Used when dev=true.
     */
    class SvelteComponentDev extends SvelteComponent {
        constructor(options) {
            if (!options || (!options.target && !options.$$inline)) {
                throw new Error("'target' is a required option");
            }
            super();
        }
        $destroy() {
            super.$destroy();
            this.$destroy = () => {
                console.warn('Component was already destroyed'); // eslint-disable-line no-console
            };
        }
        $capture_state() { }
        $inject_state() { }
    }

    /*  ------------------------------------------------------------------------ */

    var _slicedToArray = function () { function sliceIterator(arr, i) { var _arr = []; var _n = true; var _d = false; var _e = undefined; try { for (var _i = arr[Symbol.iterator](), _s; !(_n = (_s = _i.next()).done); _n = true) { _arr.push(_s.value); if (i && _arr.length === i) break; } } catch (err) { _d = true; _e = err; } finally { try { if (!_n && _i["return"]) _i["return"](); } finally { if (_d) throw _e; } } return _arr; } return function (arr, i) { if (Array.isArray(arr)) { return arr; } else if (Symbol.iterator in Object(arr)) { return sliceIterator(arr, i); } else { throw new TypeError("Invalid attempt to destructure non-iterable instance"); } }; }();

    var _createClass = function () { function defineProperties(target, props) { for (var i = 0; i < props.length; i++) { var descriptor = props[i]; descriptor.enumerable = descriptor.enumerable || false; descriptor.configurable = true; if ("value" in descriptor) descriptor.writable = true; Object.defineProperty(target, descriptor.key, descriptor); } } return function (Constructor, protoProps, staticProps) { if (protoProps) defineProperties(Constructor.prototype, protoProps); if (staticProps) defineProperties(Constructor, staticProps); return Constructor; }; }();

    function _toConsumableArray(arr) { if (Array.isArray(arr)) { for (var i = 0, arr2 = Array(arr.length); i < arr.length; i++) { arr2[i] = arr[i]; } return arr2; } else { return Array.from(arr); } }

    function _classCallCheck(instance, Constructor) { if (!(instance instanceof Constructor)) { throw new TypeError("Cannot call a class as a function"); } }

    var O = Object;

    /*  See https://misc.flogisoft.com/bash/tip_colors_and_formatting
        ------------------------------------------------------------------------ */

    var colorCodes = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'lightGray', '', 'default'],
        colorCodesLight = ['darkGray', 'lightRed', 'lightGreen', 'lightYellow', 'lightBlue', 'lightMagenta', 'lightCyan', 'white', ''],
        styleCodes = ['', 'bright', 'dim', 'italic', 'underline', '', '', 'inverse'],
        asBright = { 'red': 'lightRed',
        'green': 'lightGreen',
        'yellow': 'lightYellow',
        'blue': 'lightBlue',
        'magenta': 'lightMagenta',
        'cyan': 'lightCyan',
        'black': 'darkGray',
        'lightGray': 'white' },
        types = { 0: 'style',
        2: 'unstyle',
        3: 'color',
        9: 'colorLight',
        4: 'bgColor',
        10: 'bgColorLight' },
        subtypes = { color: colorCodes,
        colorLight: colorCodesLight,
        bgColor: colorCodes,
        bgColorLight: colorCodesLight,
        style: styleCodes,
        unstyle: styleCodes

        /*  ------------------------------------------------------------------------ */

    };var clean = function clean(obj) {
        for (var k in obj) {
            if (!obj[k]) {
                delete obj[k];
            }
        }
        return O.keys(obj).length === 0 ? undefined : obj;
    };

    /*  ------------------------------------------------------------------------ */

    var Color = function () {
        function Color(background, name, brightness) {
            _classCallCheck(this, Color);

            this.background = background;
            this.name = name;
            this.brightness = brightness;
        }

        _createClass(Color, [{
            key: 'defaultBrightness',
            value: function defaultBrightness(value) {

                return new Color(this.background, this.name, this.brightness || value);
            }
        }, {
            key: 'css',
            value: function css(inverted) {

                var color = inverted ? this.inverse : this;

                var rgbName = color.brightness === Code.bright && asBright[color.name] || color.name;

                var prop = color.background ? 'background:' : 'color:',
                    rgb = Colors.rgb[rgbName],
                    alpha = this.brightness === Code.dim ? 0.5 : 1;

                return rgb ? prop + 'rgba(' + [].concat(_toConsumableArray(rgb), [alpha]).join(',') + ');' : !color.background && alpha < 1 ? 'color:rgba(0,0,0,0.5);' : ''; // Chrome does not support 'opacity' property...
            }
        }, {
            key: 'inverse',
            get: function get() {
                return new Color(!this.background, this.name || (this.background ? 'black' : 'white'), this.brightness);
            }
        }, {
            key: 'clean',
            get: function get() {
                return clean({ name: this.name === 'default' ? '' : this.name,
                    bright: this.brightness === Code.bright,
                    dim: this.brightness === Code.dim });
            }
        }]);

        return Color;
    }();

    /*  ------------------------------------------------------------------------ */

    var Code = function () {
        function Code(n) {
            _classCallCheck(this, Code);

            if (n !== undefined) {
                this.value = Number(n);
            }
        }

        _createClass(Code, [{
            key: 'type',
            get: function get() {
                return types[Math.floor(this.value / 10)];
            }
        }, {
            key: 'subtype',
            get: function get() {
                return subtypes[this.type][this.value % 10];
            }
        }, {
            key: 'str',
            get: function get() {
                return this.value ? '\x1B[' + this.value + 'm' : '';
            }
        }, {
            key: 'isBrightness',
            get: function get() {
                return this.value === Code.noBrightness || this.value === Code.bright || this.value === Code.dim;
            }
        }], [{
            key: 'str',
            value: function str(x) {
                return new Code(x).str;
            }
        }]);

        return Code;
    }();

    /*  ------------------------------------------------------------------------ */

    O.assign(Code, {

        reset: 0,
        bright: 1,
        dim: 2,
        inverse: 7,
        noBrightness: 22,
        noItalic: 23,
        noUnderline: 24,
        noInverse: 27,
        noColor: 39,
        noBgColor: 49
    });

    /*  ------------------------------------------------------------------------ */

    var replaceAll = function replaceAll(str, a, b) {
        return str.split(a).join(b);
    };

    /*  ANSI brightness codes do not overlap, e.g. "{bright}{dim}foo" will be rendered bright (not dim).
        So we fix it by adding brightness canceling before each brightness code, so the former example gets
        converted to "{noBrightness}{bright}{noBrightness}{dim}foo" – this way it gets rendered as expected.
     */

    var denormalizeBrightness = function denormalizeBrightness(s) {
        return s.replace(/(\u001b\[(1|2)m)/g, '\x1B[22m$1');
    };
    var normalizeBrightness = function normalizeBrightness(s) {
        return s.replace(/\u001b\[22m(\u001b\[(1|2)m)/g, '$1');
    };

    var wrap = function wrap(x, openCode, closeCode) {

        var open = Code.str(openCode),
            close = Code.str(closeCode);

        return String(x).split('\n').map(function (line) {
            return denormalizeBrightness(open + replaceAll(normalizeBrightness(line), close, open) + close);
        }).join('\n');
    };

    /*  ------------------------------------------------------------------------ */

    var camel = function camel(a, b) {
        return a + b.charAt(0).toUpperCase() + b.slice(1);
    };

    var stringWrappingMethods = function () {
        return [].concat(_toConsumableArray(colorCodes.map(function (k, i) {
            return !k ? [] : [// color methods

            [k, 30 + i, Code.noColor], [camel('bg', k), 40 + i, Code.noBgColor]];
        })), _toConsumableArray(colorCodesLight.map(function (k, i) {
            return !k ? [] : [// light color methods

            [k, 90 + i, Code.noColor], [camel('bg', k), 100 + i, Code.noBgColor]];
        })), _toConsumableArray(['', 'BrightRed', 'BrightGreen', 'BrightYellow', 'BrightBlue', 'BrightMagenta', 'BrightCyan'].map(function (k, i) {
            return !k ? [] : [['bg' + k, 100 + i, Code.noBgColor]];
        })), _toConsumableArray(styleCodes.map(function (k, i) {
            return !k ? [] : [// style methods

            [k, i, k === 'bright' || k === 'dim' ? Code.noBrightness : 20 + i]];
        }))).reduce(function (a, b) {
            return a.concat(b);
        });
    }();

    /*  ------------------------------------------------------------------------ */

    var assignStringWrappingAPI = function assignStringWrappingAPI(target) {
        var wrapBefore = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : target;
        return stringWrappingMethods.reduce(function (memo, _ref) {
            var _ref2 = _slicedToArray(_ref, 3),
                k = _ref2[0],
                open = _ref2[1],
                close = _ref2[2];

            return O.defineProperty(memo, k, {
                get: function get() {
                    return assignStringWrappingAPI(function (str) {
                        return wrapBefore(wrap(str, open, close));
                    });
                }
            });
        }, target);
    };

    /*  ------------------------------------------------------------------------ */

    var TEXT = 0,
        BRACKET = 1,
        CODE = 2;

    function rawParse(s) {

        var state = TEXT,
            buffer = '',
            text = '',
            code = '',
            codes = [];
        var spans = [];

        for (var i = 0, n = s.length; i < n; i++) {

            var c = s[i];

            buffer += c;

            switch (state) {

                case TEXT:
                    if (c === '\x1B') {
                        state = BRACKET;buffer = c;
                    } else {
                        text += c;
                    }
                    break;

                case BRACKET:
                    if (c === '[') {
                        state = CODE;code = '';codes = [];
                    } else {
                        state = TEXT;text += buffer;
                    }
                    break;

                case CODE:

                    if (c >= '0' && c <= '9') {
                        code += c;
                    } else if (c === ';') {
                        codes.push(new Code(code));code = '';
                    } else if (c === 'm') {
                        code = code || '0';
                        codes.push(new Code(code));
                        var _iteratorNormalCompletion = true;
                        var _didIteratorError = false;
                        var _iteratorError = undefined;

                        try {
                            for (var _iterator = codes[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true) {
                                var _code = _step.value;
                                spans.push({ text: text, code: _code });text = '';
                            }
                        } catch (err) {
                            _didIteratorError = true;
                            _iteratorError = err;
                        } finally {
                            try {
                                if (!_iteratorNormalCompletion && _iterator.return) {
                                    _iterator.return();
                                }
                            } finally {
                                if (_didIteratorError) {
                                    throw _iteratorError;
                                }
                            }
                        }

                        state = TEXT;
                    } else {
                        state = TEXT;text += buffer;
                    }
            }
        }

        if (state !== TEXT) text += buffer;

        if (text) spans.push({ text: text, code: new Code() });

        return spans;
    }

    /*  ------------------------------------------------------------------------ */

    /**
     * Represents an ANSI-escaped string.
     */

    var Colors = function () {

        /**
         * @param {string} s a string containing ANSI escape codes.
         */
        function Colors(s) {
            _classCallCheck(this, Colors);

            this.spans = s ? rawParse(s) : [];
        }

        _createClass(Colors, [{
            key: Symbol.iterator,


            /**
             * @example
             * const spans = [...ansi.parse ('\u001b[7m\u001b[7mfoo\u001b[7mbar\u001b[27m')]
             */
            value: function value() {
                return this.spans[Symbol.iterator]();
            }

            /**
             * @desc This allows an alternative import style, see https://github.com/xpl/ansicolor/issues/7#issuecomment-578923578
             * @example
             * import { ansicolor, ParsedSpan } from 'ansicolor'
             */

        }, {
            key: 'str',
            get: function get() {
                return this.spans.reduce(function (str, p) {
                    return str + p.text + p.code.str;
                }, '');
            }
        }, {
            key: 'parsed',
            get: function get() {

                var color = void 0,
                    bgColor = void 0,
                    brightness = void 0,
                    styles = void 0;

                function reset() {

                    color = new Color(), bgColor = new Color(true /* background */), brightness = undefined, styles = new Set();
                }

                reset();

                return O.assign(new Colors(), {

                    spans: this.spans.map(function (span) {

                        var c = span.code;

                        var inverted = styles.has('inverse'),
                            underline = styles.has('underline') ? 'text-decoration: underline;' : '',
                            italic = styles.has('italic') ? 'font-style: italic;' : '',
                            bold = brightness === Code.bright ? 'font-weight: bold;' : '';

                        var foreColor = color.defaultBrightness(brightness);

                        var styledSpan = O.assign({ css: bold + italic + underline + foreColor.css(inverted) + bgColor.css(inverted) }, clean({ bold: !!bold, color: foreColor.clean, bgColor: bgColor.clean }), span);

                        var _iteratorNormalCompletion2 = true;
                        var _didIteratorError2 = false;
                        var _iteratorError2 = undefined;

                        try {
                            for (var _iterator2 = styles[Symbol.iterator](), _step2; !(_iteratorNormalCompletion2 = (_step2 = _iterator2.next()).done); _iteratorNormalCompletion2 = true) {
                                var k = _step2.value;
                                styledSpan[k] = true;
                            }
                        } catch (err) {
                            _didIteratorError2 = true;
                            _iteratorError2 = err;
                        } finally {
                            try {
                                if (!_iteratorNormalCompletion2 && _iterator2.return) {
                                    _iterator2.return();
                                }
                            } finally {
                                if (_didIteratorError2) {
                                    throw _iteratorError2;
                                }
                            }
                        }

                        if (c.isBrightness) {

                            brightness = c.value;
                        } else if (span.code.value !== undefined) {

                            if (span.code.value === Code.reset) {
                                reset();
                            } else {

                                switch (span.code.type) {

                                    case 'color':
                                    case 'colorLight':
                                        color = new Color(false, c.subtype);break;

                                    case 'bgColor':
                                    case 'bgColorLight':
                                        bgColor = new Color(true, c.subtype);break;

                                    case 'style':
                                        styles.add(c.subtype);break;
                                    case 'unstyle':
                                        styles.delete(c.subtype);break;
                                }
                            }
                        }

                        return styledSpan;
                    }).filter(function (s) {
                        return s.text.length > 0;
                    })
                });
            }

            /*  Outputs with Chrome DevTools-compatible format     */

        }, {
            key: 'asChromeConsoleLogArguments',
            get: function get() {

                var spans = this.parsed.spans;

                return [spans.map(function (s) {
                    return '%c' + s.text;
                }).join('')].concat(_toConsumableArray(spans.map(function (s) {
                    return s.css;
                })));
            }
        }, {
            key: 'browserConsoleArguments',
            get: function get() /* LEGACY, DEPRECATED */{
                return this.asChromeConsoleLogArguments;
            }

            /**
             * @desc installs String prototype extensions
             * @example
             * require ('ansicolor').nice
             * console.log ('foo'.bright.red)
             */

        }], [{
            key: 'parse',


            /**
             * @desc parses a string containing ANSI escape codes
             * @return {Colors} parsed representation.
             */
            value: function parse(s) {
                return new Colors(s).parsed;
            }

            /**
             * @desc strips ANSI codes from a string
             * @param {string} s a string containing ANSI escape codes.
             * @return {string} clean string.
             */

        }, {
            key: 'strip',
            value: function strip(s) {
                return s.replace(/[\u001b\u009b][[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-PRZcf-nqry=><]/g, ''); // hope V8 caches the regexp
            }

            /**
             * @desc checks if a value contains ANSI escape codes
             * @param {any} s value to check
             * @return {boolean} has codes
             */

        }, {
            key: 'isEscaped',
            value: function isEscaped(s) {
                s = String(s);
                return Colors.strip(s) !== s;
            }
        }, {
            key: 'nice',
            get: function get() {

                Colors.names.forEach(function (k) {
                    if (!(k in String.prototype)) {
                        O.defineProperty(String.prototype, k, { get: function get() {
                                return Colors[k](this);
                            } });
                    }
                });

                return Colors;
            }
        }, {
            key: 'ansicolor',
            get: function get() {
                return Colors;
            }
        }]);

        return Colors;
    }();

    /*  ------------------------------------------------------------------------ */

    assignStringWrappingAPI(Colors, function (str) {
        return str;
    });

    /*  ------------------------------------------------------------------------ */

    Colors.names = stringWrappingMethods.map(function (_ref3) {
        var _ref4 = _slicedToArray(_ref3, 1),
            k = _ref4[0];

        return k;
    });

    /*  ------------------------------------------------------------------------ */

    Colors.rgb = {

        black: [0, 0, 0],
        darkGray: [100, 100, 100],
        lightGray: [200, 200, 200],
        white: [255, 255, 255],

        red: [204, 0, 0],
        lightRed: [255, 51, 0],

        green: [0, 204, 0],
        lightGreen: [51, 204, 51],

        yellow: [204, 102, 0],
        lightYellow: [255, 153, 51],

        blue: [0, 0, 255],
        lightBlue: [26, 140, 255],

        magenta: [204, 0, 204],
        lightMagenta: [255, 0, 255],

        cyan: [0, 153, 255],
        lightCyan: [0, 204, 255]

        /*  ------------------------------------------------------------------------ */

    };var ansicolor = Colors;

    /* Regenerate in Repl:
      const Colors = require('ansicolor')
      const reserved = [...Object.getOwnPropertyNames(Object), 'default']
      const filtered = Object.getOwnPropertyNames(Colors).filter(k => !reserved.includes(k) )
      filtered.forEach(k => console.log(`export const ${k} = Colors.${k}`))
      console.log('export default Colors')

    */

    ansicolor.nice;
    ansicolor.parse;
    const strip = ansicolor.strip;
    ansicolor.ansicolor;
    ansicolor.black;
    ansicolor.bgBlack;
    ansicolor.red;
    ansicolor.bgRed;
    ansicolor.green;
    ansicolor.bgGreen;
    ansicolor.yellow;
    ansicolor.bgYellow;
    ansicolor.blue;
    ansicolor.bgBlue;
    ansicolor.magenta;
    ansicolor.bgMagenta;
    ansicolor.cyan;
    ansicolor.bgCyan;
    ansicolor.lightGray;
    ansicolor.bgLightGray;
    ansicolor.bgDefault;
    ansicolor.darkGray;
    ansicolor.bgDarkGray;
    ansicolor.lightRed;
    ansicolor.bgLightRed;
    ansicolor.lightGreen;
    ansicolor.bgLightGreen;
    ansicolor.lightYellow;
    ansicolor.bgLightYellow;
    ansicolor.lightBlue;
    ansicolor.bgLightBlue;
    ansicolor.lightMagenta;
    ansicolor.bgLightMagenta;
    ansicolor.lightCyan;
    ansicolor.bgLightCyan;
    ansicolor.white;
    ansicolor.bgWhite;
    ansicolor.bgBrightRed;
    ansicolor.bgBrightGreen;
    ansicolor.bgBrightYellow;
    ansicolor.bgBrightBlue;
    ansicolor.bgBrightMagenta;
    ansicolor.bgBrightCyan;
    ansicolor.bright;
    ansicolor.dim;
    ansicolor.italic;
    ansicolor.underline;
    ansicolor.inverse;
    ansicolor.names;
    ansicolor.rgb;

    const subscriber_queue = [];
    /**
     * Creates a `Readable` store that allows reading by subscription.
     * @param value initial value
     * @param {StartStopNotifier}start start and stop notifications for subscriptions
     */
    function readable(value, start) {
        return {
            subscribe: writable(value, start).subscribe
        };
    }
    /**
     * Create a `Writable` store that allows both updating and reading by subscription.
     * @param {*=}value initial value
     * @param {StartStopNotifier=}start start and stop notifications for subscriptions
     */
    function writable(value, start = noop) {
        let stop;
        const subscribers = new Set();
        function set(new_value) {
            if (safe_not_equal(value, new_value)) {
                value = new_value;
                if (stop) { // store is ready
                    const run_queue = !subscriber_queue.length;
                    for (const subscriber of subscribers) {
                        subscriber[1]();
                        subscriber_queue.push(subscriber, value);
                    }
                    if (run_queue) {
                        for (let i = 0; i < subscriber_queue.length; i += 2) {
                            subscriber_queue[i][0](subscriber_queue[i + 1]);
                        }
                        subscriber_queue.length = 0;
                    }
                }
            }
        }
        function update(fn) {
            set(fn(value));
        }
        function subscribe(run, invalidate = noop) {
            const subscriber = [run, invalidate];
            subscribers.add(subscriber);
            if (subscribers.size === 1) {
                stop = start(set) || noop;
            }
            run(value);
            return () => {
                subscribers.delete(subscriber);
                if (subscribers.size === 0) {
                    stop();
                    stop = null;
                }
            };
        }
        return { set, update, subscribe };
    }
    function derived(stores, fn, initial_value) {
        const single = !Array.isArray(stores);
        const stores_array = single
            ? [stores]
            : stores;
        const auto = fn.length < 2;
        return readable(initial_value, (set) => {
            let inited = false;
            const values = [];
            let pending = 0;
            let cleanup = noop;
            const sync = () => {
                if (pending) {
                    return;
                }
                cleanup();
                const result = fn(single ? values[0] : values, set);
                if (auto) {
                    set(result);
                }
                else {
                    cleanup = is_function(result) ? result : noop;
                }
            };
            const unsubscribers = stores_array.map((store, i) => subscribe$1(store, (value) => {
                values[i] = value;
                pending &= ~(1 << i);
                if (inited) {
                    sync();
                }
            }, () => {
                pending |= (1 << i);
            }));
            inited = true;
            sync();
            return function stop() {
                run_all(unsubscribers);
                cleanup();
            };
        });
    }

    const rez = writable([]);
    const logs = writable([]);
    const nodes = writable({});
    const tag = writable("");
    const connected = writable(false);
    const info = derived([nodes, tag], ([$nodes, $tag]) => {
        let idx = $nodes[$tag];
        return {
            peering: `cln${idx}:${9735 + idx}`,
            broker: `${IP}:${1883 + idx}`,
            control: 5000 + idx,
            grpc: 10019 + idx,
        };
    });
    const IS_DEV = window.location.host === "localhost:8080";
    const DEV_TAG = "sphinx-6We";
    const IP = "44.211.127.45";
    let root = "/api";
    if (IS_DEV) {
        root = "http://localhost:8000/api";
    }
    async function send_cmd(txt) {
        let ctag = get_store_value(tag);
        if (!ctag)
            return console.error("not logged in");
        const r = await fetch(`${root}/cmd?txt=${txt}&tag=${ctag}`);
        const newtxt = await r.text();
        const txts = newtxt.split("\n").filter((a) => a);
        if (txts.length) {
            rez.update((r) => [...txts, ...r]);
        }
        return newtxt;
    }
    fetch(`nodes.json`)
        .then((r) => r.json())
        .then((data) => {
        nodes.set(data);
        if (IS_DEV)
            login(DEV_TAG);
    });
    async function login(nn) {
        if (!nn.includes("-"))
            return false;
        const n = nn.split("-")[1];
        if (!n)
            return false;
        const current_nodes = get_store_value(nodes);
        if (current_nodes[n]) {
            tag.set(n);
            await get_logs(n);
            logstream(n);
            return true;
        }
        return false;
    }
    async function get_logs(tag) {
        const r = await fetch(`${root}/logs?tag=${tag}`);
        const lg = await r.json();
        if (Array.isArray(lg)) {
            let cleanlogs = lg.map((l) => strip(l).trim());
            logs.set(cleanlogs.reverse());
        }
        return lg;
    }
    function logstream(tag) {
        subscribe(`${root}/logstream?tag=${tag}`, (msg) => {
            logs.update((r) => [strip(msg), ...r]);
        });
    }
    function subscribe(uri, cb) {
        var retryTime = 1;
        function connect(uri) {
            const events = new EventSource(uri);
            events.addEventListener("message", (ev) => {
                try {
                    let dat = JSON.parse(ev.data);
                    cb(dat.trim());
                }
                catch (e) {
                    console.log("could parse incoming msg", e);
                }
            });
            events.addEventListener("open", () => {
                connected.set(true);
            });
            events.addEventListener("error", () => {
                connected.set(false);
                events.close();
                let timeout = retryTime;
                retryTime = Math.min(64, retryTime * 2);
                console.log(`connection lost. attempting to reconnect in ${timeout}s`);
                setTimeout(() => connect(uri), (() => timeout * 1000)());
            });
        }
        connect(uri);
    }

    /* src/Btn.svelte generated by Svelte v3.50.1 */

    const file$3 = "src/Btn.svelte";

    function create_fragment$3(ctx) {
    	let div;
    	let t;
    	let mounted;
    	let dispose;

    	const block = {
    		c: function create() {
    			div = element("div");
    			t = text(/*content*/ ctx[0]);
    			attr_dev(div, "style", /*style*/ ctx[1]);
    			attr_dev(div, "class", "svelte-6iqo24");
    			add_location(div, file$3, 5, 0, 72);
    		},
    		l: function claim(nodes) {
    			throw new Error("options.hydrate only works if the component was compiled with the `hydratable: true` option");
    		},
    		m: function mount(target, anchor) {
    			insert_dev(target, div, anchor);
    			append_dev(div, t);

    			if (!mounted) {
    				dispose = listen_dev(div, "click", /*click_handler*/ ctx[2], false, false, false);
    				mounted = true;
    			}
    		},
    		p: function update(ctx, [dirty]) {
    			if (dirty & /*content*/ 1) set_data_dev(t, /*content*/ ctx[0]);

    			if (dirty & /*style*/ 2) {
    				attr_dev(div, "style", /*style*/ ctx[1]);
    			}
    		},
    		i: noop,
    		o: noop,
    		d: function destroy(detaching) {
    			if (detaching) detach_dev(div);
    			mounted = false;
    			dispose();
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_fragment$3.name,
    		type: "component",
    		source: "",
    		ctx
    	});

    	return block;
    }

    function instance$3($$self, $$props, $$invalidate) {
    	let { $$slots: slots = {}, $$scope } = $$props;
    	validate_slots('Btn', slots, []);
    	let { content = "" } = $$props;
    	let { style = "" } = $$props;
    	const writable_props = ['content', 'style'];

    	Object.keys($$props).forEach(key => {
    		if (!~writable_props.indexOf(key) && key.slice(0, 2) !== '$$' && key !== 'slot') console.warn(`<Btn> was created with unknown prop '${key}'`);
    	});

    	function click_handler(event) {
    		bubble.call(this, $$self, event);
    	}

    	$$self.$$set = $$props => {
    		if ('content' in $$props) $$invalidate(0, content = $$props.content);
    		if ('style' in $$props) $$invalidate(1, style = $$props.style);
    	};

    	$$self.$capture_state = () => ({ content, style });

    	$$self.$inject_state = $$props => {
    		if ('content' in $$props) $$invalidate(0, content = $$props.content);
    		if ('style' in $$props) $$invalidate(1, style = $$props.style);
    	};

    	if ($$props && "$$inject" in $$props) {
    		$$self.$inject_state($$props.$$inject);
    	}

    	return [content, style, click_handler];
    }

    class Btn extends SvelteComponentDev {
    	constructor(options) {
    		super(options);
    		init(this, options, instance$3, create_fragment$3, safe_not_equal, { content: 0, style: 1 });

    		dispatch_dev("SvelteRegisterComponent", {
    			component: this,
    			tagName: "Btn",
    			options,
    			id: create_fragment$3.name
    		});
    	}

    	get content() {
    		throw new Error("<Btn>: Props cannot be read directly from the component instance unless compiling with 'accessors: true' or '<svelte:options accessors/>'");
    	}

    	set content(value) {
    		throw new Error("<Btn>: Props cannot be set directly on the component instance unless compiling with 'accessors: true' or '<svelte:options accessors/>'");
    	}

    	get style() {
    		throw new Error("<Btn>: Props cannot be read directly from the component instance unless compiling with 'accessors: true' or '<svelte:options accessors/>'");
    	}

    	set style(value) {
    		throw new Error("<Btn>: Props cannot be set directly on the component instance unless compiling with 'accessors: true' or '<svelte:options accessors/>'");
    	}
    }

    const cmds = [
        "$CLN getinfo",
        "$CLN newaddr",
        "export FRIEND_PUBKEY=xxx",
        "export FRIEND_ADDR=cln3:9738",
        "$CLN connect $FRIEND_PUBKEY $FRIEND_ADDR",
    ];

    /* src/Cmd.svelte generated by Svelte v3.50.1 */

    const file$2 = "src/Cmd.svelte";

    // (26:2) {#if label}
    function create_if_block$2(ctx) {
    	let span;
    	let t;

    	const block = {
    		c: function create() {
    			span = element("span");
    			t = text(/*label*/ ctx[0]);
    			attr_dev(span, "class", "svelte-1cjg4lc");
    			add_location(span, file$2, 25, 13, 475);
    		},
    		m: function mount(target, anchor) {
    			insert_dev(target, span, anchor);
    			append_dev(span, t);
    		},
    		p: function update(ctx, dirty) {
    			if (dirty & /*label*/ 1) set_data_dev(t, /*label*/ ctx[0]);
    		},
    		d: function destroy(detaching) {
    			if (detaching) detach_dev(span);
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_if_block$2.name,
    		type: "if",
    		source: "(26:2) {#if label}",
    		ctx
    	});

    	return block;
    }

    function create_fragment$2(ctx) {
    	let div;
    	let svg;
    	let path;
    	let svg_style_value;
    	let t0;
    	let t1;
    	let span;
    	let t2;
    	let mounted;
    	let dispose;
    	let if_block = /*label*/ ctx[0] && create_if_block$2(ctx);

    	const block = {
    		c: function create() {
    			div = element("div");
    			svg = svg_element("svg");
    			path = svg_element("path");
    			t0 = space();
    			if (if_block) if_block.c();
    			t1 = space();
    			span = element("span");
    			t2 = text(/*cmd*/ ctx[1]);
    			attr_dev(path, "d", "M22 6v16h-16v-16h16zm2-2h-20v20h20v-20zm-24 17v-21h21v2h-19v19h-2z");
    			add_location(path, file$2, 21, 4, 363);
    			attr_dev(svg, "class", "copy svelte-1cjg4lc");
    			attr_dev(svg, "viewBox", "0 0 24 24");
    			attr_dev(svg, "style", svg_style_value = `transform:scale(${/*scale*/ ctx[2]},${/*scale*/ ctx[2]})`);
    			add_location(svg, file$2, 15, 2, 240);
    			attr_dev(span, "class", "svelte-1cjg4lc");
    			add_location(span, file$2, 26, 2, 503);
    			attr_dev(div, "class", "cmd svelte-1cjg4lc");
    			add_location(div, file$2, 14, 0, 220);
    		},
    		l: function claim(nodes) {
    			throw new Error("options.hydrate only works if the component was compiled with the `hydratable: true` option");
    		},
    		m: function mount(target, anchor) {
    			insert_dev(target, div, anchor);
    			append_dev(div, svg);
    			append_dev(svg, path);
    			append_dev(div, t0);
    			if (if_block) if_block.m(div, null);
    			append_dev(div, t1);
    			append_dev(div, span);
    			append_dev(span, t2);

    			if (!mounted) {
    				dispose = listen_dev(svg, "click", /*copy*/ ctx[3], false, false, false);
    				mounted = true;
    			}
    		},
    		p: function update(ctx, [dirty]) {
    			if (dirty & /*scale*/ 4 && svg_style_value !== (svg_style_value = `transform:scale(${/*scale*/ ctx[2]},${/*scale*/ ctx[2]})`)) {
    				attr_dev(svg, "style", svg_style_value);
    			}

    			if (/*label*/ ctx[0]) {
    				if (if_block) {
    					if_block.p(ctx, dirty);
    				} else {
    					if_block = create_if_block$2(ctx);
    					if_block.c();
    					if_block.m(div, t1);
    				}
    			} else if (if_block) {
    				if_block.d(1);
    				if_block = null;
    			}

    			if (dirty & /*cmd*/ 2) set_data_dev(t2, /*cmd*/ ctx[1]);
    		},
    		i: noop,
    		o: noop,
    		d: function destroy(detaching) {
    			if (detaching) detach_dev(div);
    			if (if_block) if_block.d();
    			mounted = false;
    			dispose();
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_fragment$2.name,
    		type: "component",
    		source: "",
    		ctx
    	});

    	return block;
    }

    function instance$2($$self, $$props, $$invalidate) {
    	let { $$slots: slots = {}, $$scope } = $$props;
    	validate_slots('Cmd', slots, []);
    	let { label = "" } = $$props;
    	let { cmd = "" } = $$props;
    	let scale = 1;

    	function copy() {
    		$$invalidate(2, scale = 1.4);
    		navigator.clipboard.writeText(cmd);

    		setTimeout(
    			() => {
    				$$invalidate(2, scale = 1);
    			},
    			101
    		);
    	}

    	const writable_props = ['label', 'cmd'];

    	Object.keys($$props).forEach(key => {
    		if (!~writable_props.indexOf(key) && key.slice(0, 2) !== '$$' && key !== 'slot') console.warn(`<Cmd> was created with unknown prop '${key}'`);
    	});

    	$$self.$$set = $$props => {
    		if ('label' in $$props) $$invalidate(0, label = $$props.label);
    		if ('cmd' in $$props) $$invalidate(1, cmd = $$props.cmd);
    	};

    	$$self.$capture_state = () => ({ label, cmd, scale, copy });

    	$$self.$inject_state = $$props => {
    		if ('label' in $$props) $$invalidate(0, label = $$props.label);
    		if ('cmd' in $$props) $$invalidate(1, cmd = $$props.cmd);
    		if ('scale' in $$props) $$invalidate(2, scale = $$props.scale);
    	};

    	if ($$props && "$$inject" in $$props) {
    		$$self.$inject_state($$props.$$inject);
    	}

    	return [label, cmd, scale, copy];
    }

    class Cmd extends SvelteComponentDev {
    	constructor(options) {
    		super(options);
    		init(this, options, instance$2, create_fragment$2, safe_not_equal, { label: 0, cmd: 1 });

    		dispatch_dev("SvelteRegisterComponent", {
    			component: this,
    			tagName: "Cmd",
    			options,
    			id: create_fragment$2.name
    		});
    	}

    	get label() {
    		throw new Error("<Cmd>: Props cannot be read directly from the component instance unless compiling with 'accessors: true' or '<svelte:options accessors/>'");
    	}

    	set label(value) {
    		throw new Error("<Cmd>: Props cannot be set directly on the component instance unless compiling with 'accessors: true' or '<svelte:options accessors/>'");
    	}

    	get cmd() {
    		throw new Error("<Cmd>: Props cannot be read directly from the component instance unless compiling with 'accessors: true' or '<svelte:options accessors/>'");
    	}

    	set cmd(value) {
    		throw new Error("<Cmd>: Props cannot be set directly on the component instance unless compiling with 'accessors: true' or '<svelte:options accessors/>'");
    	}
    }

    /* src/App.svelte generated by Svelte v3.50.1 */
    const file$1 = "src/App.svelte";

    function get_each_context(ctx, list, i) {
    	const child_ctx = ctx.slice();
    	child_ctx[11] = list[i];
    	return child_ctx;
    }

    function get_each_context_1(ctx, list, i) {
    	const child_ctx = ctx.slice();
    	child_ctx[14] = list[i];
    	return child_ctx;
    }

    function get_each_context_2(ctx, list, i) {
    	const child_ctx = ctx.slice();
    	child_ctx[17] = list[i];
    	return child_ctx;
    }

    // (34:4) {#each $logs as log}
    function create_each_block_2(ctx) {
    	let div;
    	let t_value = /*log*/ ctx[17] + "";
    	let t;

    	const block = {
    		c: function create() {
    			div = element("div");
    			t = text(t_value);
    			attr_dev(div, "class", "log svelte-opghj6");
    			add_location(div, file$1, 34, 6, 743);
    		},
    		m: function mount(target, anchor) {
    			insert_dev(target, div, anchor);
    			append_dev(div, t);
    		},
    		p: function update(ctx, dirty) {
    			if (dirty & /*$logs*/ 8 && t_value !== (t_value = /*log*/ ctx[17] + "")) set_data_dev(t, t_value);
    		},
    		d: function destroy(detaching) {
    			if (detaching) detach_dev(div);
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_each_block_2.name,
    		type: "each",
    		source: "(34:4) {#each $logs as log}",
    		ctx
    	});

    	return block;
    }

    // (42:4) {#each $rez as term}
    function create_each_block_1(ctx) {
    	let div;
    	let t_value = /*term*/ ctx[14] + "";
    	let t;

    	const block = {
    		c: function create() {
    			div = element("div");
    			t = text(t_value);
    			attr_dev(div, "class", "log svelte-opghj6");
    			add_location(div, file$1, 42, 6, 869);
    		},
    		m: function mount(target, anchor) {
    			insert_dev(target, div, anchor);
    			append_dev(div, t);
    		},
    		p: function update(ctx, dirty) {
    			if (dirty & /*$rez*/ 16 && t_value !== (t_value = /*term*/ ctx[14] + "")) set_data_dev(t, t_value);
    		},
    		d: function destroy(detaching) {
    			if (detaching) detach_dev(div);
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_each_block_1.name,
    		type: "each",
    		source: "(42:4) {#each $rez as term}",
    		ctx
    	});

    	return block;
    }

    // (56:0) {#if help}
    function create_if_block$1(ctx) {
    	let section;
    	let h50;
    	let t1;
    	let div0;
    	let t2;
    	let cmd0;
    	let t3;
    	let cmd1;
    	let t4;
    	let div1;
    	let t5;
    	let h51;
    	let t7;
    	let div2;
    	let t8;
    	let current;

    	cmd0 = new Cmd({
    			props: {
    				label: "Peering Address:",
    				cmd: /*$info*/ ctx[5].peering
    			},
    			$$inline: true
    		});

    	cmd1 = new Cmd({
    			props: {
    				label: "MQTT Broker:",
    				cmd: /*$info*/ ctx[5].broker
    			},
    			$$inline: true
    		});

    	let each_value = cmds;
    	validate_each_argument(each_value);
    	let each_blocks = [];

    	for (let i = 0; i < each_value.length; i += 1) {
    		each_blocks[i] = create_each_block(get_each_context(ctx, each_value, i));
    	}

    	const out = i => transition_out(each_blocks[i], 1, 1, () => {
    		each_blocks[i] = null;
    	});

    	const block = {
    		c: function create() {
    			section = element("section");
    			h50 = element("h5");
    			h50.textContent = "Node info";
    			t1 = space();
    			div0 = element("div");
    			t2 = space();
    			create_component(cmd0.$$.fragment);
    			t3 = space();
    			create_component(cmd1.$$.fragment);
    			t4 = space();
    			div1 = element("div");
    			t5 = space();
    			h51 = element("h5");
    			h51.textContent = "Command Examples";
    			t7 = space();
    			div2 = element("div");
    			t8 = space();

    			for (let i = 0; i < each_blocks.length; i += 1) {
    				each_blocks[i].c();
    			}

    			attr_dev(h50, "class", "svelte-opghj6");
    			add_location(h50, file$1, 57, 4, 1169);
    			attr_dev(div0, "class", "break svelte-opghj6");
    			add_location(div0, file$1, 58, 4, 1192);
    			attr_dev(div1, "class", "break svelte-opghj6");
    			add_location(div1, file$1, 61, 4, 1327);
    			attr_dev(h51, "class", "svelte-opghj6");
    			add_location(h51, file$1, 62, 4, 1353);
    			attr_dev(div2, "class", "break svelte-opghj6");
    			add_location(div2, file$1, 63, 4, 1383);
    			attr_dev(section, "class", "help-section svelte-opghj6");
    			add_location(section, file$1, 56, 2, 1134);
    		},
    		m: function mount(target, anchor) {
    			insert_dev(target, section, anchor);
    			append_dev(section, h50);
    			append_dev(section, t1);
    			append_dev(section, div0);
    			append_dev(section, t2);
    			mount_component(cmd0, section, null);
    			append_dev(section, t3);
    			mount_component(cmd1, section, null);
    			append_dev(section, t4);
    			append_dev(section, div1);
    			append_dev(section, t5);
    			append_dev(section, h51);
    			append_dev(section, t7);
    			append_dev(section, div2);
    			append_dev(section, t8);

    			for (let i = 0; i < each_blocks.length; i += 1) {
    				each_blocks[i].m(section, null);
    			}

    			current = true;
    		},
    		p: function update(ctx, dirty) {
    			const cmd0_changes = {};
    			if (dirty & /*$info*/ 32) cmd0_changes.cmd = /*$info*/ ctx[5].peering;
    			cmd0.$set(cmd0_changes);
    			const cmd1_changes = {};
    			if (dirty & /*$info*/ 32) cmd1_changes.cmd = /*$info*/ ctx[5].broker;
    			cmd1.$set(cmd1_changes);

    			if (dirty & /*cmds*/ 0) {
    				each_value = cmds;
    				validate_each_argument(each_value);
    				let i;

    				for (i = 0; i < each_value.length; i += 1) {
    					const child_ctx = get_each_context(ctx, each_value, i);

    					if (each_blocks[i]) {
    						each_blocks[i].p(child_ctx, dirty);
    						transition_in(each_blocks[i], 1);
    					} else {
    						each_blocks[i] = create_each_block(child_ctx);
    						each_blocks[i].c();
    						transition_in(each_blocks[i], 1);
    						each_blocks[i].m(section, null);
    					}
    				}

    				group_outros();

    				for (i = each_value.length; i < each_blocks.length; i += 1) {
    					out(i);
    				}

    				check_outros();
    			}
    		},
    		i: function intro(local) {
    			if (current) return;
    			transition_in(cmd0.$$.fragment, local);
    			transition_in(cmd1.$$.fragment, local);

    			for (let i = 0; i < each_value.length; i += 1) {
    				transition_in(each_blocks[i]);
    			}

    			current = true;
    		},
    		o: function outro(local) {
    			transition_out(cmd0.$$.fragment, local);
    			transition_out(cmd1.$$.fragment, local);
    			each_blocks = each_blocks.filter(Boolean);

    			for (let i = 0; i < each_blocks.length; i += 1) {
    				transition_out(each_blocks[i]);
    			}

    			current = false;
    		},
    		d: function destroy(detaching) {
    			if (detaching) detach_dev(section);
    			destroy_component(cmd0);
    			destroy_component(cmd1);
    			destroy_each(each_blocks, detaching);
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_if_block$1.name,
    		type: "if",
    		source: "(56:0) {#if help}",
    		ctx
    	});

    	return block;
    }

    // (65:4) {#each cmds as cmd}
    function create_each_block(ctx) {
    	let cmd;
    	let current;

    	cmd = new Cmd({
    			props: { cmd: /*cmd*/ ctx[11] },
    			$$inline: true
    		});

    	const block = {
    		c: function create() {
    			create_component(cmd.$$.fragment);
    		},
    		m: function mount(target, anchor) {
    			mount_component(cmd, target, anchor);
    			current = true;
    		},
    		p: noop,
    		i: function intro(local) {
    			if (current) return;
    			transition_in(cmd.$$.fragment, local);
    			current = true;
    		},
    		o: function outro(local) {
    			transition_out(cmd.$$.fragment, local);
    			current = false;
    		},
    		d: function destroy(detaching) {
    			destroy_component(cmd, detaching);
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_each_block.name,
    		type: "each",
    		source: "(65:4) {#each cmds as cmd}",
    		ctx
    	});

    	return block;
    }

    function create_fragment$1(ctx) {
    	let section0;
    	let h50;
    	let t1;
    	let p0;
    	let t2;
    	let section1;
    	let h51;
    	let t4;
    	let p1;
    	let t5;
    	let div;
    	let textarea_1;
    	let t6;
    	let span;
    	let t8;
    	let t9;
    	let btn;
    	let current;
    	let mounted;
    	let dispose;
    	let each_value_2 = /*$logs*/ ctx[3];
    	validate_each_argument(each_value_2);
    	let each_blocks_1 = [];

    	for (let i = 0; i < each_value_2.length; i += 1) {
    		each_blocks_1[i] = create_each_block_2(get_each_context_2(ctx, each_value_2, i));
    	}

    	let each_value_1 = /*$rez*/ ctx[4];
    	validate_each_argument(each_value_1);
    	let each_blocks = [];

    	for (let i = 0; i < each_value_1.length; i += 1) {
    		each_blocks[i] = create_each_block_1(get_each_context_1(ctx, each_value_1, i));
    	}

    	let if_block = /*help*/ ctx[2] && create_if_block$1(ctx);

    	btn = new Btn({
    			props: {
    				content: /*help*/ ctx[2] ? "X" : "</>",
    				style: "position:absolute;top:3px;right:1rem;"
    			},
    			$$inline: true
    		});

    	btn.$on("click", /*click_handler*/ ctx[9]);

    	const block = {
    		c: function create() {
    			section0 = element("section");
    			h50 = element("h5");
    			h50.textContent = "Core Lightning Logs";
    			t1 = space();
    			p0 = element("p");

    			for (let i = 0; i < each_blocks_1.length; i += 1) {
    				each_blocks_1[i].c();
    			}

    			t2 = space();
    			section1 = element("section");
    			h51 = element("h5");
    			h51.textContent = "Terminal";
    			t4 = space();
    			p1 = element("p");

    			for (let i = 0; i < each_blocks.length; i += 1) {
    				each_blocks[i].c();
    			}

    			t5 = space();
    			div = element("div");
    			textarea_1 = element("textarea");
    			t6 = space();
    			span = element("span");
    			span.textContent = "$";
    			t8 = space();
    			if (if_block) if_block.c();
    			t9 = space();
    			create_component(btn.$$.fragment);
    			attr_dev(h50, "class", "svelte-opghj6");
    			add_location(h50, file$1, 31, 2, 677);
    			attr_dev(p0, "class", "svelte-opghj6");
    			add_location(p0, file$1, 32, 2, 708);
    			attr_dev(section0, "class", "svelte-opghj6");
    			add_location(section0, file$1, 30, 0, 665);
    			attr_dev(h51, "class", "svelte-opghj6");
    			add_location(h51, file$1, 39, 2, 814);
    			attr_dev(p1, "class", "svelte-opghj6");
    			add_location(p1, file$1, 40, 2, 834);
    			attr_dev(textarea_1, "placeholder", "Type commands here");
    			attr_dev(textarea_1, "class", "svelte-opghj6");
    			add_location(textarea_1, file$1, 46, 4, 947);
    			attr_dev(span, "class", "svelte-opghj6");
    			add_location(span, file$1, 52, 4, 1086);
    			attr_dev(div, "class", "txt-wrap svelte-opghj6");
    			add_location(div, file$1, 45, 2, 920);
    			attr_dev(section1, "class", "svelte-opghj6");
    			add_location(section1, file$1, 38, 0, 802);
    		},
    		l: function claim(nodes) {
    			throw new Error("options.hydrate only works if the component was compiled with the `hydratable: true` option");
    		},
    		m: function mount(target, anchor) {
    			insert_dev(target, section0, anchor);
    			append_dev(section0, h50);
    			append_dev(section0, t1);
    			append_dev(section0, p0);

    			for (let i = 0; i < each_blocks_1.length; i += 1) {
    				each_blocks_1[i].m(p0, null);
    			}

    			insert_dev(target, t2, anchor);
    			insert_dev(target, section1, anchor);
    			append_dev(section1, h51);
    			append_dev(section1, t4);
    			append_dev(section1, p1);

    			for (let i = 0; i < each_blocks.length; i += 1) {
    				each_blocks[i].m(p1, null);
    			}

    			append_dev(section1, t5);
    			append_dev(section1, div);
    			append_dev(div, textarea_1);
    			/*textarea_1_binding*/ ctx[7](textarea_1);
    			set_input_value(textarea_1, /*txt*/ ctx[1]);
    			append_dev(div, t6);
    			append_dev(div, span);
    			insert_dev(target, t8, anchor);
    			if (if_block) if_block.m(target, anchor);
    			insert_dev(target, t9, anchor);
    			mount_component(btn, target, anchor);
    			current = true;

    			if (!mounted) {
    				dispose = [
    					listen_dev(textarea_1, "input", /*textarea_1_input_handler*/ ctx[8]),
    					listen_dev(textarea_1, "keypress", /*keypress*/ ctx[6], false, false, false)
    				];

    				mounted = true;
    			}
    		},
    		p: function update(ctx, [dirty]) {
    			if (dirty & /*$logs*/ 8) {
    				each_value_2 = /*$logs*/ ctx[3];
    				validate_each_argument(each_value_2);
    				let i;

    				for (i = 0; i < each_value_2.length; i += 1) {
    					const child_ctx = get_each_context_2(ctx, each_value_2, i);

    					if (each_blocks_1[i]) {
    						each_blocks_1[i].p(child_ctx, dirty);
    					} else {
    						each_blocks_1[i] = create_each_block_2(child_ctx);
    						each_blocks_1[i].c();
    						each_blocks_1[i].m(p0, null);
    					}
    				}

    				for (; i < each_blocks_1.length; i += 1) {
    					each_blocks_1[i].d(1);
    				}

    				each_blocks_1.length = each_value_2.length;
    			}

    			if (dirty & /*$rez*/ 16) {
    				each_value_1 = /*$rez*/ ctx[4];
    				validate_each_argument(each_value_1);
    				let i;

    				for (i = 0; i < each_value_1.length; i += 1) {
    					const child_ctx = get_each_context_1(ctx, each_value_1, i);

    					if (each_blocks[i]) {
    						each_blocks[i].p(child_ctx, dirty);
    					} else {
    						each_blocks[i] = create_each_block_1(child_ctx);
    						each_blocks[i].c();
    						each_blocks[i].m(p1, null);
    					}
    				}

    				for (; i < each_blocks.length; i += 1) {
    					each_blocks[i].d(1);
    				}

    				each_blocks.length = each_value_1.length;
    			}

    			if (dirty & /*txt*/ 2) {
    				set_input_value(textarea_1, /*txt*/ ctx[1]);
    			}

    			if (/*help*/ ctx[2]) {
    				if (if_block) {
    					if_block.p(ctx, dirty);

    					if (dirty & /*help*/ 4) {
    						transition_in(if_block, 1);
    					}
    				} else {
    					if_block = create_if_block$1(ctx);
    					if_block.c();
    					transition_in(if_block, 1);
    					if_block.m(t9.parentNode, t9);
    				}
    			} else if (if_block) {
    				group_outros();

    				transition_out(if_block, 1, 1, () => {
    					if_block = null;
    				});

    				check_outros();
    			}

    			const btn_changes = {};
    			if (dirty & /*help*/ 4) btn_changes.content = /*help*/ ctx[2] ? "X" : "</>";
    			btn.$set(btn_changes);
    		},
    		i: function intro(local) {
    			if (current) return;
    			transition_in(if_block);
    			transition_in(btn.$$.fragment, local);
    			current = true;
    		},
    		o: function outro(local) {
    			transition_out(if_block);
    			transition_out(btn.$$.fragment, local);
    			current = false;
    		},
    		d: function destroy(detaching) {
    			if (detaching) detach_dev(section0);
    			destroy_each(each_blocks_1, detaching);
    			if (detaching) detach_dev(t2);
    			if (detaching) detach_dev(section1);
    			destroy_each(each_blocks, detaching);
    			/*textarea_1_binding*/ ctx[7](null);
    			if (detaching) detach_dev(t8);
    			if (if_block) if_block.d(detaching);
    			if (detaching) detach_dev(t9);
    			destroy_component(btn, detaching);
    			mounted = false;
    			run_all(dispose);
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_fragment$1.name,
    		type: "component",
    		source: "",
    		ctx
    	});

    	return block;
    }

    function instance$1($$self, $$props, $$invalidate) {
    	let $logs;
    	let $rez;
    	let $info;
    	validate_store(logs, 'logs');
    	component_subscribe($$self, logs, $$value => $$invalidate(3, $logs = $$value));
    	validate_store(rez, 'rez');
    	component_subscribe($$self, rez, $$value => $$invalidate(4, $rez = $$value));
    	validate_store(info, 'info');
    	component_subscribe($$self, info, $$value => $$invalidate(5, $info = $$value));
    	let { $$slots: slots = {}, $$scope } = $$props;
    	validate_slots('App', slots, []);
    	let textarea;

    	onMount(function () {
    		textarea.focus();
    	});

    	function send(txt) {
    		if (txt === "clear\n") {
    			return rez.set([]);
    		}

    		rez.update(r => [`$ ${txt}`, ...r]);
    		send_cmd(txt);
    	}

    	let txt = "";

    	function keypress(e) {
    		if (e.key === "Enter") {
    			send(txt);
    			$$invalidate(1, txt = "");
    			e.stopPropagation();

    			setTimeout(
    				() => {
    					e.target.setSelectionRange(0, 0);
    				},
    				1
    			);
    		}
    	}

    	let help = false;
    	const writable_props = [];

    	Object.keys($$props).forEach(key => {
    		if (!~writable_props.indexOf(key) && key.slice(0, 2) !== '$$' && key !== 'slot') console.warn(`<App> was created with unknown prop '${key}'`);
    	});

    	function textarea_1_binding($$value) {
    		binding_callbacks[$$value ? 'unshift' : 'push'](() => {
    			textarea = $$value;
    			$$invalidate(0, textarea);
    		});
    	}

    	function textarea_1_input_handler() {
    		txt = this.value;
    		$$invalidate(1, txt);
    	}

    	const click_handler = () => $$invalidate(2, help = !help);

    	$$self.$capture_state = () => ({
    		rez,
    		send_cmd,
    		logs,
    		info,
    		Btn,
    		cmds,
    		Cmd,
    		onMount,
    		textarea,
    		send,
    		txt,
    		keypress,
    		help,
    		$logs,
    		$rez,
    		$info
    	});

    	$$self.$inject_state = $$props => {
    		if ('textarea' in $$props) $$invalidate(0, textarea = $$props.textarea);
    		if ('txt' in $$props) $$invalidate(1, txt = $$props.txt);
    		if ('help' in $$props) $$invalidate(2, help = $$props.help);
    	};

    	if ($$props && "$$inject" in $$props) {
    		$$self.$inject_state($$props.$$inject);
    	}

    	return [
    		textarea,
    		txt,
    		help,
    		$logs,
    		$rez,
    		$info,
    		keypress,
    		textarea_1_binding,
    		textarea_1_input_handler,
    		click_handler
    	];
    }

    class App extends SvelteComponentDev {
    	constructor(options) {
    		super(options);
    		init(this, options, instance$1, create_fragment$1, safe_not_equal, {});

    		dispatch_dev("SvelteRegisterComponent", {
    			component: this,
    			tagName: "App",
    			options,
    			id: create_fragment$1.name
    		});
    	}
    }

    /* src/Wrap.svelte generated by Svelte v3.50.1 */

    const { console: console_1 } = globals;
    const file = "src/Wrap.svelte";

    // (19:2) {:else}
    function create_else_block(ctx) {
    	let input;
    	let mounted;
    	let dispose;

    	const block = {
    		c: function create() {
    			input = element("input");
    			attr_dev(input, "placeholder", "Password");
    			attr_dev(input, "class", "svelte-s2m3w3");
    			add_location(input, file, 19, 4, 340);
    		},
    		m: function mount(target, anchor) {
    			insert_dev(target, input, anchor);
    			set_input_value(input, /*ctag*/ ctx[0]);

    			if (!mounted) {
    				dispose = [
    					listen_dev(input, "input", /*input_input_handler*/ ctx[3]),
    					listen_dev(input, "keypress", /*keypress*/ ctx[2], false, false, false)
    				];

    				mounted = true;
    			}
    		},
    		p: function update(ctx, dirty) {
    			if (dirty & /*ctag*/ 1 && input.value !== /*ctag*/ ctx[0]) {
    				set_input_value(input, /*ctag*/ ctx[0]);
    			}
    		},
    		i: noop,
    		o: noop,
    		d: function destroy(detaching) {
    			if (detaching) detach_dev(input);
    			mounted = false;
    			run_all(dispose);
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_else_block.name,
    		type: "else",
    		source: "(19:2) {:else}",
    		ctx
    	});

    	return block;
    }

    // (17:2) {#if $tag}
    function create_if_block(ctx) {
    	let app;
    	let current;
    	app = new App({ $$inline: true });

    	const block = {
    		c: function create() {
    			create_component(app.$$.fragment);
    		},
    		m: function mount(target, anchor) {
    			mount_component(app, target, anchor);
    			current = true;
    		},
    		p: noop,
    		i: function intro(local) {
    			if (current) return;
    			transition_in(app.$$.fragment, local);
    			current = true;
    		},
    		o: function outro(local) {
    			transition_out(app.$$.fragment, local);
    			current = false;
    		},
    		d: function destroy(detaching) {
    			destroy_component(app, detaching);
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_if_block.name,
    		type: "if",
    		source: "(17:2) {#if $tag}",
    		ctx
    	});

    	return block;
    }

    function create_fragment(ctx) {
    	let main;
    	let current_block_type_index;
    	let if_block;
    	let current;
    	const if_block_creators = [create_if_block, create_else_block];
    	const if_blocks = [];

    	function select_block_type(ctx, dirty) {
    		if (/*$tag*/ ctx[1]) return 0;
    		return 1;
    	}

    	current_block_type_index = select_block_type(ctx);
    	if_block = if_blocks[current_block_type_index] = if_block_creators[current_block_type_index](ctx);

    	const block = {
    		c: function create() {
    			main = element("main");
    			if_block.c();
    			attr_dev(main, "class", "svelte-s2m3w3");
    			add_location(main, file, 15, 0, 294);
    		},
    		l: function claim(nodes) {
    			throw new Error("options.hydrate only works if the component was compiled with the `hydratable: true` option");
    		},
    		m: function mount(target, anchor) {
    			insert_dev(target, main, anchor);
    			if_blocks[current_block_type_index].m(main, null);
    			current = true;
    		},
    		p: function update(ctx, [dirty]) {
    			let previous_block_index = current_block_type_index;
    			current_block_type_index = select_block_type(ctx);

    			if (current_block_type_index === previous_block_index) {
    				if_blocks[current_block_type_index].p(ctx, dirty);
    			} else {
    				group_outros();

    				transition_out(if_blocks[previous_block_index], 1, 1, () => {
    					if_blocks[previous_block_index] = null;
    				});

    				check_outros();
    				if_block = if_blocks[current_block_type_index];

    				if (!if_block) {
    					if_block = if_blocks[current_block_type_index] = if_block_creators[current_block_type_index](ctx);
    					if_block.c();
    				} else {
    					if_block.p(ctx, dirty);
    				}

    				transition_in(if_block, 1);
    				if_block.m(main, null);
    			}
    		},
    		i: function intro(local) {
    			if (current) return;
    			transition_in(if_block);
    			current = true;
    		},
    		o: function outro(local) {
    			transition_out(if_block);
    			current = false;
    		},
    		d: function destroy(detaching) {
    			if (detaching) detach_dev(main);
    			if_blocks[current_block_type_index].d();
    		}
    	};

    	dispatch_dev("SvelteRegisterBlock", {
    		block,
    		id: create_fragment.name,
    		type: "component",
    		source: "",
    		ctx
    	});

    	return block;
    }

    function instance($$self, $$props, $$invalidate) {
    	let $tag;
    	validate_store(tag, 'tag');
    	component_subscribe($$self, tag, $$value => $$invalidate(1, $tag = $$value));
    	let { $$slots: slots = {}, $$scope } = $$props;
    	validate_slots('Wrap', slots, []);
    	let ctag = "";

    	async function keypress(e) {
    		if (e.key === "Enter") {
    			if (!await login(ctag)) {
    				console.error("invalid password");
    			}
    		}
    	}

    	const writable_props = [];

    	Object.keys($$props).forEach(key => {
    		if (!~writable_props.indexOf(key) && key.slice(0, 2) !== '$$' && key !== 'slot') console_1.warn(`<Wrap> was created with unknown prop '${key}'`);
    	});

    	function input_input_handler() {
    		ctag = this.value;
    		$$invalidate(0, ctag);
    	}

    	$$self.$capture_state = () => ({ App, tag, login, ctag, keypress, $tag });

    	$$self.$inject_state = $$props => {
    		if ('ctag' in $$props) $$invalidate(0, ctag = $$props.ctag);
    	};

    	if ($$props && "$$inject" in $$props) {
    		$$self.$inject_state($$props.$$inject);
    	}

    	return [ctag, $tag, keypress, input_input_handler];
    }

    class Wrap extends SvelteComponentDev {
    	constructor(options) {
    		super(options);
    		init(this, options, instance, create_fragment, safe_not_equal, {});

    		dispatch_dev("SvelteRegisterComponent", {
    			component: this,
    			tagName: "Wrap",
    			options,
    			id: create_fragment.name
    		});
    	}
    }

    const app = new Wrap({
        target: document.body,
    });

    return app;

})();
//# sourceMappingURL=bundle.js.map
