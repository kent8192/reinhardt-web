<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{title}} - Reinhardt API</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { padding: 20px; border-bottom: 1px solid #e0e0e0; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; border-radius: 8px 8px 0 0; }
        .header h1 { margin: 0 0 10px 0; font-size: 24px; }
        .header p { margin: 0; opacity: 0.9; }
        .content { padding: 20px; }
        .method-badge { display: inline-block; padding: 4px 12px; border-radius: 4px; font-weight: bold; font-size: 12px; margin-right: 10px; }
        .method-get { background: #4caf50; color: white; }
        .method-post { background: #2196f3; color: white; }
        .method-put { background: #ff9800; color: white; }
        .method-patch { background: #9c27b0; color: white; }
        .method-delete { background: #f44336; color: white; }
        .endpoint { font-family: monospace; background: #f5f5f5; padding: 8px 12px; border-radius: 4px; display: inline-block; margin: 10px 0; }
        .response { background: #263238; color: #aed581; padding: 20px; border-radius: 4px; overflow-x: auto; margin: 20px 0; }
        .response pre { margin: 0; white-space: pre-wrap; word-wrap: break-word; }
        .form-section { margin: 20px 0; padding: 20px; background: #f9f9f9; border-radius: 4px; }
        .form-field { margin-bottom: 15px; }
        .form-field label { display: block; margin-bottom: 5px; font-weight: 500; }
        .form-field input, .form-field textarea, .form-field select { width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px; }
        .form-field textarea { min-height: 100px; font-family: monospace; }
        .help-text { font-size: 12px; color: #666; margin-top: 4px; }
        .submit-btn { background: #667eea; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; font-size: 14px; font-weight: 500; }
        .submit-btn:hover { background: #5568d3; }
        .allowed-methods { margin: 15px 0; }
        .allowed-methods span { margin-right: 10px; }
        .headers { margin: 20px 0; }
        .headers table { width: 100%; border-collapse: collapse; }
        .headers th, .headers td { text-align: left; padding: 8px; border-bottom: 1px solid #e0e0e0; }
        .headers th { font-weight: 500; background: #f5f5f5; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>{{title}}</h1>
            {{#if description}}<p>{{description}}</p>{{/if}}
        </div>

        <div class="content">
            <div class="allowed-methods">
                <strong>Allowed methods:</strong>
                {{#each allowed_methods}}
                <span class="method-badge method-{{this}}">{{this}}</span>
                {{/each}}
            </div>

            <div class="endpoint">
                <span class="method-badge method-{{method}}">{{method}}</span>
                {{endpoint}}
            </div>

            <h2>Response ({{response_status}})</h2>
            <div class="response">
                <pre>{{response_data_formatted}}</pre>
            </div>

            {{#if request_form}}
            <div class="form-section">
                <h2>Make a Request</h2>
                <form method="{{request_form.submit_method}}" action="{{request_form.submit_url}}">
                    {{#each request_form.fields}}
                    <div class="form-field">
                        <label for="{{name}}">
                            {{label}}
                            {{#if required}}<span style="color: red;">*</span>{{/if}}
                        </label>
                        {{#if (eq field_type "select")}}
                        <select id="{{name}}" name="{{name}}" {{#if required}}required{{/if}}>
                            {{#if initial_label}}
                            <option value="" selected>{{initial_label}}</option>
                            {{/if}}
                            {{#each options}}
                            <option value="{{value}}" {{#if (eq value ../initial_value)}}selected{{/if}}>{{label}}</option>
                            {{/each}}
                        </select>
                        {{else if (eq field_type "textarea")}}
                        <textarea id="{{name}}" name="{{name}}" {{#if required}}required{{/if}}>{{#if initial_value}}{{initial_value}}{{/if}}</textarea>
                        {{else}}
                        <input type="{{field_type}}" id="{{name}}" name="{{name}}" {{#if required}}required{{/if}} {{#if initial_value}}value="{{initial_value}}"{{/if}}>
                        {{/if}}
                        {{#if help_text}}<div class="help-text">{{help_text}}</div>{{/if}}
                    </div>
                    {{/each}}
                    <button type="submit" class="submit-btn">Submit</button>
                </form>
            </div>
            {{/if}}

            {{#if headers}}
            <div class="headers">
                <h2>Response Headers</h2>
                <table>
                    <thead>
                        <tr>
                            <th>Header</th>
                            <th>Value</th>
                        </tr>
                    </thead>
                    <tbody>
                        {{#each headers}}
                        <tr>
                            <td><strong>{{this.0}}</strong></td>
                            <td>{{this.1}}</td>
                        </tr>
                        {{/each}}
                    </tbody>
                </table>
            </div>
            {{/if}}
        </div>
    </div>
</body>
</html>
