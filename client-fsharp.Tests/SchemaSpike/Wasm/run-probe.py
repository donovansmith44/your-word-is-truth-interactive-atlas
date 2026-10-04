import argparse
import json
import time
from pathlib import Path
from playwright.sync_api import sync_playwright


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:5100/')
    parser.add_argument('--output', required=True)
    parser.add_argument('--browser')
    parser.add_argument('--invalidate-root', action='store_true')
    parser.add_argument('--expect', choices=['passed', 'failed'], default='passed')
    arguments = parser.parse_args()
    events = []
    errors = []
    failures = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=arguments.browser)
        page = browser.new_page()
        page.on('console', lambda message: events.append({'type': message.type, 'text': message.text}))
        page.on('pageerror', lambda error: errors.append(str(error)))
        page.on('response', lambda response: failures.append({'url': response.url, 'status': response.status}) if response.status >= 400 else None)
        if arguments.invalidate_root:
            page.route('**/survey.json', invalidate_root)
        started = time.monotonic()
        page.goto(arguments.url, wait_until='networkidle', timeout=60000)
        try:
            page.wait_for_function("['passed','failed'].includes(document.querySelector('#status')?.textContent)", timeout=45000)
        except Exception as error:
            errors.append(str(error))
        status = page.locator('#status').inner_text() if page.locator('#status').count() else 'missing'
        report = page.locator('#report').inner_text() if page.locator('#report').count() else None
        resources = page.evaluate('performance.getEntriesByType("resource").map(r => ({name:r.name, transferred:r.transferSize, encoded:r.encodedBodySize, decoded:r.decodedBodySize, duration:r.duration}))')
        result = {'url': arguments.url, 'status': status, 'report': json.loads(report) if status == 'passed' else report,
                  'elapsed_seconds': time.monotonic() - started, 'resources': resources, 'console': events, 'page_errors': errors, 'http_failures': failures,
                  'control': 'invalid-root' if arguments.invalidate_root else 'original-inputs'}
        browser.close()
    output = Path(arguments.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'status': status, 'report': result['report'], 'page_errors': errors}))
    return 0 if status == arguments.expect and not errors and not failures else 1


def invalidate_root(route):
    response = route.fetch()
    body = response.json()
    example = next(example for example in body['Examples'] if 'version' in example['Body'])
    example['Body']['version'] = 'root-0'
    route.fulfill(response=response, json=body)


if __name__ == '__main__':
    raise SystemExit(main())
