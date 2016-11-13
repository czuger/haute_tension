json.extract! page_link, :id, :text, :src_page, :dst_page, :created_at, :updated_at
json.url page_link_url(page_link, format: :json)