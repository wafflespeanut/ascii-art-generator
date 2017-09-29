from PIL import Image
from StringIO import StringIO
from flask import Flask, json, render_template, request
from gen import Sketch
from werkzeug import secure_filename

import cgi
import os
import random
import string
import urllib2

UPLOAD_FOLDER = './'
ALLOWED_EXTENSIONS = set(['jpg', 'jpeg', 'png'])
MAX_UPLOAD_SIZE = 4 * 1024 * 1024

DEFAULT_FONT_SIZE_PX = 4
DEFAULT_LINE_HEIGHT = 1
DEFAULT_MIN_LEVEL = 78
DEFAULT_MAX_LEVEL = 125
DEFAULT_GAMMA = 0.78


def render_index(**kwargs):
    kwargs.setdefault('lh', DEFAULT_LINE_HEIGHT)
    kwargs.setdefault('fs', DEFAULT_FONT_SIZE_PX)
    kwargs.setdefault('minl', DEFAULT_MIN_LEVEL)
    kwargs.setdefault('maxl', DEFAULT_MAX_LEVEL)
    kwargs.setdefault('gamma', DEFAULT_GAMMA)
    kwargs.setdefault('name', 'unknown')
    return render_template('index.html', **kwargs)


def open_image(path_or_fd, path=True):
    img = None
    try:
        img = Image.open(path_or_fd)
    except Exception:
        pass

    if path:
        os.remove(path_or_fd)
    if not img:
        raise Exception('Error processing image!')
    return img


def get_image(url):
    if not url:
        raise Exception('Expected file or URL!')
    try:
        print 'Downloading %s...' % url
        fd = urllib2.urlopen(url)
        size = int(fd.info().getheaders("Content-Length")[0])
        if size > 4 * MAX_UPLOAD_SIZE:      # 16 MB for URL
            raise Exception('Max. allowed size for image: 4 MB')
    except Exception as err:
        print 'Error downloading %s: %s' % (url, err)
        raise Exception('Cannot download file from URL!')

    return open_image(StringIO(fd.read()), path=False)


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


if __name__ == '__main__':
    app = Flask('ASCII Art Generator')
    app.config['UPLOAD_FOLDER'] = UPLOAD_FOLDER
    app.config['ALLOWED_EXTENSIONS'] = ALLOWED_EXTENSIONS
    app.config['MAX_CONTENT_LENGTH'] = MAX_UPLOAD_SIZE

    sketch = Sketch()


    def jsonify(data, code=400):
        return app.response_class(
            response=json.dumps(data),
            status=code,
            mimetype='application/json'
        )


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
            image = get_image(url)
        except Exception as err:
            return render_index(error=err[0])

        try:
            lines = sketch.generate_ascii(image, required_width=width,
                                          min_level=min_l, max_level=max_l, gamma=gamma)
            art = ''.join('<div class="ascii-line">' + cgi.escape(line) + '</div>' for line in lines)
        except Exception as err:
            print 'ERROR: %s' % err
            return render_index(err='Image unsupported!')

        return render_index(art=art, special=True, fs=4, lh=1)


    def allowed_file(filename):
        return '.' in filename and filename.rsplit('.', 1)[1].lower() in ALLOWED_EXTENSIONS


    @app.route('/', methods=['POST'])
    def upload_file():
        image = None
        min_l = request.form.get('min_level', DEFAULT_MIN_LEVEL)
        max_l = request.form.get('max_level', DEFAULT_MAX_LEVEL)
        gamma = request.form.get('gamma', DEFAULT_GAMMA)

        try:
            min_l, max_l, gamma, _ = parse_params(min_l, max_l, gamma)
            url = request.form.get('url')
            if url:
                image = get_image(url)
            else:
                if 'file' not in request.files:
                    return jsonify({'error': 'Only files or URL allowed!'})
                f = request.files['file']
                if f.filename == '':
                    return jsonify({'error': 'Expected file or URL!'})
                elif not allowed_file(f.filename):
                    return jsonify({'error': 'Unsupported file!'})
                else:
                    img_path = os.path.join(app.config['UPLOAD_FOLDER'], secure_filename(f.filename))
                    f.save(img_path)
                    image = open_image(img_path)
        except Exception as err:
            return jsonify({'error': err[0]})

        lines = sketch.generate_ascii(image, min_level=min_l, max_level=max_l, gamma=gamma)
        return jsonify({'art': [cgi.escape(line) for line in lines]}, code=200)


    port = int(os.environ.get('PORT', 5000))
    app.run(host='0.0.0.0', port=port)
