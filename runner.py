from flask import Flask, json, render_template, request
from gen import generate_art

import os
import random
import string
import urllib2

UPLOAD_FOLDER = './'
ALLOWED_EXTENSIONS = set(['jpg', 'jpeg', 'png'])

DEFAULT_FONT_SIZE_PX = 4
DEFAULT_LINE_HEIGHT = 1
DEFAULT_MIN_LEVEL = 78
DEFAULT_MAX_LEVEL = 125
DEFAULT_GAMMA = 0.78

def random_name(length):
    return ''.join(random.choice(string.lowercase) for i in range(length))

def allowed_file(filename):
    return '.' in filename and filename.rsplit('.', 1)[1].lower() in ALLOWED_EXTENSIONS

app = Flask('ASCII Art Generator')
app.config['UPLOAD_FOLDER'] = UPLOAD_FOLDER
app.config['ALLOWED_EXTENSIONS'] = ALLOWED_EXTENSIONS
app.config['MAX_CONTENT_LENGTH'] = 4 * 1024 * 1024      # 4 MB

def render_index(**kwargs):
    kwargs.setdefault('lh', DEFAULT_LINE_HEIGHT)
    kwargs.setdefault('fs', DEFAULT_FONT_SIZE_PX)
    kwargs.setdefault('minl', DEFAULT_MIN_LEVEL)
    kwargs.setdefault('maxl', DEFAULT_MAX_LEVEL)
    kwargs.setdefault('gamma', DEFAULT_GAMMA)
    kwargs.setdefault('name', 'unknown')
    return render_template('index.html', **kwargs)

def jsonify(data):
    return app.response_class(
        response=json.dumps(data),
        status=200,
        mimetype='application/json'
    )

def download_image(url):
    if not url:
        raise Exception('Expected file or URL!')
    try:
        filename = random_name(20)
        fd = urllib2.urlopen(url)
        with open(filename, 'wb') as img:
            img.write(fd.read())
        return filename
    except Exception as err:
        print 'Error downloading %s: %s' % (url, err)
        raise Exception('Cannot download file from URL!')

def parse_params(min_l, max_l, gamma, width=None):
    try:
        min_l = int(min_l)
        assert min_l >=0 and min_l <= 255, 'Min level should be in [0, 255]'
        max_l = int(max_l)
        assert max_l >=0 and max_l <= 255 and max_l > min_l, 'Max level should be in [MIN, 255]'
        gamma = float(gamma)
        assert gamma > 0.0 and gamma <= 1.0, 'Gamma value should be in (0, 1]'
        if width is not None:
            width = int(width)
        return min_l, max_l, gamma, width
    except ValueError:
        raise Exception('Invalid parameter(s) supplied!')
    except AssertionError as err:
        return Exception(err)

@app.route('/', methods=['GET'])
def index():
    url = request.args.get('url')
    if not url:
        return render_index()

    width = request.args.get('w')
    minl = request.args.get('min', DEFAULT_MIN_LEVEL)
    maxl = request.args.get('max', DEFAULT_MAX_LEVEL)
    gamma = request.args.get('g', DEFAULT_GAMMA)
    try:
        min_l, max_l, gamma, width = parse_params(minl, maxl, gamma, width)
        filename = download_image(url)
    except Exception as err:
        return render_index(error=err)

    try:
        lines = generate_art(filename, given_width=width, html=True,
                             min_level=min_l, max_level=max_l, gamma=gamma)
        art = ''.join(map(lambda line: '<div class="ascii-line">' + line + '</div>', lines))
    except Exception:
        os.remove(filename)
        return jsonify({'error': 'Image unsupported!'})

    os.remove(filename)
    return render_index(art=art, special=True, fs=4, lh=1)

@app.route('/', methods=['POST'])
def upload_file():
    filename = None
    min_l = request.form.get('min_level', DEFAULT_MIN_LEVEL)
    max_l = request.form.get('max_level', DEFAULT_MAX_LEVEL)
    gamma = request.form.get('gamma', DEFAULT_GAMMA)

    try:
        min_l, max_l, gamma, _ = parse_params(min_l, max_l, gamma)
        url = request.form.get('url')
        if url:
            filename = download_image(url)
        else:
            if 'file' not in request.files:
                return jsonify({'error': 'Only files or URL allowed!'})
            f = request.files['file']
            if f.filename == '':
                return jsonify({'error': 'Expected file or URL!'})
            if f and allowed_file(f.filename):
                filename = random_name(20)
                f.save(os.path.join(app.config['UPLOAD_FOLDER'], filename))
    except Exception as err:
        if filename and os.path.exists(filename):
            os.remove(filename)
        return jsonify({'error': err[0]})

    try:
        art = generate_art(filename, min_level=min_l, max_level=max_l, gamma=gamma, html=True)
    except Exception:
        os.remove(filename)
        return jsonify({'error': 'Image unsupported!'})

    os.remove(filename)
    return jsonify({'art': art})

port = int(os.environ.get('PORT', 5000))
app.run(debug=True, host='0.0.0.0', port=port, threaded=True)
