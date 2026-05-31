use std::net::TcpListener;
use std::io::{Read, Write};
use std::process::Command;
use std::thread;

pub fn start_preview_server(
    markup: String,
    validation_summary: String,
    status: String,
    warnings: Vec<String>,
    errors: Vec<String>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to local port");
    let addr = listener.local_addr().unwrap();
    println!("\n=======================================================");
    println!("  Interactive Block Sandbox Preview Server");
    println!("  Running at: http://127.0.0.1:{}", addr.port());
    println!("  Press Ctrl+C to stop the server.");
    println!("=======================================================\n");

    let url = format!("http://127.0.0.1:{}", addr.port());
    let _ = Command::new("open").arg(&url).status();

    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            let markup_clone = markup.clone();
            let summary_clone = validation_summary.clone();
            let status_clone = status.clone();
            let warnings_clone = warnings.clone();
            let errors_clone = errors.clone();

            thread::spawn(move || {
                let mut buffer = [0; 1024];
                let _ = stream.read(&mut buffer);

                let request = String::from_utf8_lossy(&buffer);
                if request.starts_with("GET / ") || request.starts_with("GET /index.html ") {
                    let html = generate_preview_html(
                        &markup_clone,
                        &summary_clone,
                        &status_clone,
                        &warnings_clone,
                        &errors_clone,
                    );
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html.len(),
                        html
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                } else {
                    let not_found = "HTTP/1.1 404 NOT FOUND\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = stream.write_all(not_found.as_bytes());
                    let _ = stream.flush();
                }
            });
        }
    }
}

fn generate_preview_html(
    markup: &str,
    validation_summary: &str,
    status: &str,
    warnings: &[String],
    errors: &[String],
) -> String {
    // Determine status badge color
    let badge_class = match status {
        "verified" => "badge-verified",
        "attributes_only" => "badge-attribs",
        "structural_only" => "badge-structural",
        _ => "badge-invalid",
    };

    let status_text = match status {
        "verified" => "VERIFIED",
        "attributes_only" => "VALID (Attributes only)",
        "structural_only" => "VALID (Structural only)",
        _ => "INVALID",
    };

    // Warnings list HTML
    let warnings_html = if warnings.is_empty() {
        "<div class=\"no-issues\">No warnings detected.</div>".to_string()
    } else {
        let mut html = "<ul class=\"issues-list warning-list\">".to_string();
        for w in warnings {
            html.push_str(&format!("<li><span class=\"icon\">⚠️</span>{}</li>", w));
        }
        html.push_str("</ul>");
        html
    };

    // Errors list HTML
    let errors_html = if errors.is_empty() {
        "<div class=\"no-issues\">No errors detected.</div>".to_string()
    } else {
        let mut html = "<ul class=\"issues-list error-list\">".to_string();
        for e in errors {
            html.push_str(&format!("<li><span class=\"icon\">❌</span>{}</li>", e));
        }
        html.push_str("</ul>");
        html
    };

    // Extract inner HTML for live rendering
    // We clean the comment delimiters out of the live render preview so only clean HTML remains
    let mut clean_html = markup.to_string();
    let re_comments = regex::Regex::new(r"<!--\s*/?wp:.*?\s*-->").unwrap();
    clean_html = re_comments.replace_all(&clean_html, "").to_string();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Gutenberg Block Sandbox</title>
    
    <!-- Google Fonts -->
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;600;700&family=Fira+Code:wght@400;500&display=swap" rel="stylesheet">
    
    <!-- Gutenberg standard block styles from CDN -->
    <link rel="stylesheet" href="https://unpkg.com/@wordpress/block-library@8.7.0/build-style/style.css">
    <link rel="stylesheet" href="https://unpkg.com/@wordpress/block-library@8.7.0/build-style/theme.css">

    <style>
        :root {{
            --bg-color: #0f172a;
            --card-bg: #1e293b;
            --accent-color: #6366f1;
            --accent-glow: rgba(99, 102, 241, 0.15);
            --border-color: #334155;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --color-success: #10b981;
            --color-warning: #f59e0b;
            --color-error: #ef4444;
        }}

        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }}

        body {{
            font-family: 'Outfit', sans-serif;
            background-color: var(--bg-color);
            color: var(--text-primary);
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            overflow-x: hidden;
        }}

        header {{
            background-color: rgba(30, 41, 59, 0.7);
            backdrop-filter: blur(12px);
            border-bottom: 1px solid var(--border-color);
            padding: 1rem 2rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
            position: sticky;
            top: 0;
            z-index: 10;
        }}

        .brand {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}

        .brand h1 {{
            font-size: 1.5rem;
            font-weight: 700;
            background: linear-gradient(135deg, #a5b4fc 0%, #818cf8 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}

        .brand-logo {{
            width: 32px;
            height: 32px;
            background-color: var(--accent-color);
            border-radius: 8px;
            display: grid;
            place-items: center;
            font-weight: bold;
            color: white;
            box-shadow: 0 0 15px var(--accent-glow);
        }}

        .status-badge {{
            padding: 0.4rem 1rem;
            border-radius: 50px;
            font-size: 0.85rem;
            font-weight: 600;
            letter-spacing: 0.05em;
            box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1);
        }}

        .badge-verified {{
            background-color: rgba(16, 185, 129, 0.15);
            border: 1px solid var(--color-success);
            color: var(--color-success);
        }}

        .badge-attribs {{
            background-color: rgba(99, 102, 241, 0.15);
            border: 1px solid var(--accent-color);
            color: #818cf8;
        }}

        .badge-structural {{
            background-color: rgba(245, 158, 11, 0.15);
            border: 1px solid var(--color-warning);
            color: var(--color-warning);
        }}

        .badge-invalid {{
            background-color: rgba(239, 68, 68, 0.15);
            border: 1px solid var(--color-error);
            color: var(--color-error);
        }}

        main {{
            flex: 1;
            display: grid;
            grid-template-columns: 1fr 450px;
            gap: 0;
            height: calc(100vh - 65px);
        }}

        .preview-pane {{
            padding: 3rem;
            background-color: #0b0f19;
            overflow-y: auto;
            display: flex;
            justify-content: center;
            align-items: flex-start;
        }}

        .canvas {{
            background-color: white;
            color: #333333;
            width: 100%;
            max-width: 900px;
            border-radius: 12px;
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
            padding: 2.5rem;
            min-height: 300px;
            overflow: hidden;
            /* Ensure alignment widths display inside our canvas wrapper */
        }}

        /* Sandbox specific WordPress wrapper overrides */
        .canvas .wp-block-group {{
            padding: 1.5rem;
            border: 1px dashed #dddddd;
            border-radius: 8px;
        }}

        .inspector-pane {{
            background-color: var(--card-bg);
            border-left: 1px solid var(--border-color);
            display: flex;
            flex-direction: column;
            height: 100%;
            overflow: hidden;
        }}

        .tabs {{
            display: flex;
            border-bottom: 1px solid var(--border-color);
            background-color: rgba(15, 23, 42, 0.4);
        }}

        .tab-btn {{
            flex: 1;
            padding: 1rem;
            background: none;
            border: none;
            color: var(--text-secondary);
            font-size: 0.9rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
            text-align: center;
            border-bottom: 2px solid transparent;
        }}

        .tab-btn:hover {{
            color: var(--text-primary);
            background-color: rgba(255, 255, 255, 0.02);
        }}

        .tab-btn.active {{
            color: var(--accent-color);
            border-bottom-color: var(--accent-color);
            background-color: rgba(99, 102, 241, 0.05);
        }}

        .tab-content {{
            flex: 1;
            overflow-y: auto;
            padding: 1.5rem;
            display: none;
        }}

        .tab-content.active {{
            display: block;
        }}

        h3 {{
            font-size: 1.1rem;
            font-weight: 600;
            margin-bottom: 1rem;
            color: var(--text-primary);
        }}

        .issues-list {{
            list-style: none;
            display: flex;
            flex-direction: column;
            gap: 0.75rem;
        }}

        .issues-list li {{
            padding: 0.75rem 1rem;
            border-radius: 8px;
            font-size: 0.9rem;
            line-height: 1.4;
            display: flex;
            align-items: flex-start;
            gap: 0.75rem;
        }}

        .warning-list li {{
            background-color: rgba(245, 158, 11, 0.06);
            border: 1px solid rgba(245, 158, 11, 0.2);
            color: #fcd34d;
        }}

        .error-list li {{
            background-color: rgba(239, 68, 68, 0.06);
            border: 1px solid rgba(239, 68, 68, 0.2);
            color: #fca5a5;
        }}

        .no-issues {{
            padding: 2rem;
            text-align: center;
            color: var(--text-secondary);
            border: 1px dashed var(--border-color);
            border-radius: 8px;
            font-size: 0.95rem;
        }}

        .summary-card {{
            background-color: rgba(15, 23, 42, 0.4);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 1rem;
            margin-bottom: 1.5rem;
            font-size: 0.92rem;
            line-height: 1.5;
            color: var(--text-secondary);
        }}

        .summary-card strong {{
            color: var(--text-primary);
        }}

        pre {{
            font-family: 'Fira Code', monospace;
            font-size: 0.85rem;
            background-color: #0b0f19;
            padding: 1rem;
            border-radius: 8px;
            overflow-x: auto;
            border: 1px solid var(--border-color);
            color: #38bdf8;
            max-height: 500px;
        }}

        .copy-container {{
            position: relative;
        }}

        .copy-btn {{
            position: absolute;
            top: 0.5rem;
            right: 0.5rem;
            padding: 0.35rem 0.75rem;
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 4px;
            color: var(--text-secondary);
            font-size: 0.75rem;
            cursor: pointer;
            transition: all 0.2s;
        }}

        .copy-btn:hover {{
            color: var(--text-primary);
            border-color: var(--accent-color);
        }}
    </style>
</head>
<body>

    <header>
        <div class="brand">
            <div class="brand-logo">W</div>
            <h1>Block Sandbox</h1>
        </div>
        <div class="status-badge {}">{}</div>
    </header>

    <main>
        <div class="preview-pane">
            <div class="canvas">
                {}
            </div>
        </div>
        
        <div class="inspector-pane">
            <div class="tabs">
                <button class="tab-btn active" onclick="openTab(event, 'tab-summary')">Validation Summary</button>
                <button class="tab-btn" onclick="openTab(event, 'tab-markup')">Raw Markup</button>
            </div>

            <div id="tab-summary" class="tab-content active">
                <div class="summary-card">
                    <strong>Validation Outcome:</strong><br>
                    {}
                </div>

                <h3>Errors ({})</h3>
                {}

                <h3 style="margin-top: 2rem;">Warnings ({})</h3>
                {}
            </div>

            <div id="tab-markup" class="tab-content">
                <div class="copy-container">
                    <button class="copy-btn" onclick="copyMarkup()">Copy</button>
                    <pre id="markup-code">{}</pre>
                </div>
            </div>
        </div>
    </main>

    <script>
        function openTab(evt, tabName) {{
            var i, tabcontent, tablinks;
            tabcontent = document.getElementsByClassName("tab-content");
            for (i = 0; i < tabcontent.length; i++) {{
                tabcontent[i].classList.remove("active");
            }}
            tablinks = document.getElementsByClassName("tab-btn");
            for (i = 0; i < tablinks.length; i++) {{
                tablinks[i].classList.remove("active");
            }}
            document.getElementById(tabName).classList.add("active");
            evt.currentTarget.classList.add("active");
        }}

        function copyMarkup() {{
            var codeText = document.getElementById("markup-code").innerText;
            navigator.clipboard.writeText(codeText).then(function() {{
                var btn = document.querySelector(".copy-btn");
                btn.innerText = "Copied!";
                setTimeout(function() {{
                    btn.innerText = "Copy";
                }}, 2000);
            }});
        }}
    </script>
</body>
</html>"#,
        badge_class,
        status_text,
        clean_html,
        validation_summary,
        errors.len(),
        errors_html,
        warnings.len(),
        warnings_html,
        html_escape(markup)
    )
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
