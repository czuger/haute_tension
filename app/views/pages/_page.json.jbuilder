json.extract! page, :id, :text, :url, :created_at, :updated_at
json.url page_url(page, format: :json)