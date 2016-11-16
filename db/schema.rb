# This file is auto-generated from the current state of the database. Instead
# of editing this file, please use the migrations feature of Active Record to
# incrementally modify your database, and then regenerate this schema definition.
#
# Note that this schema.rb definition is the authoritative source for your
# database schema. If you need to create the application database on another
# system, you should be using db:schema:load, not running all the migrations
# from scratch. The latter is a flawed and unsustainable approach (the more migrations
# you'll amass, the slower it'll run and the greater likelihood for issues).
#
# It's strongly recommended that you check this file into your version control system.

ActiveRecord::Schema.define(version: 20161116052349) do

  # These are extensions that must be enabled in order to support this database
  enable_extension "plpgsql"

  create_table "aventures", force: :cascade do |t|
    t.integer  "book_id",                         null: false
    t.integer  "page_id",                         null: false
    t.integer  "hp",                              null: false
    t.integer  "force",                           null: false
    t.integer  "gold",                            null: false
    t.integer  "gourdes",          default: 2,    null: false
    t.integer  "gourdes_remplies", default: 2
    t.integer  "rations",          default: 4
    t.boolean  "charisme",         default: true
    t.datetime "created_at",                      null: false
    t.datetime "updated_at",                      null: false
    t.index ["book_id"], name: "index_aventures_on_book_id", using: :btree
    t.index ["page_id"], name: "index_aventures_on_page_id", using: :btree
  end

  create_table "books", force: :cascade do |t|
    t.string   "name",          null: false
    t.integer  "first_page_id"
    t.datetime "created_at",    null: false
    t.datetime "updated_at",    null: false
    t.index ["first_page_id"], name: "index_books_on_first_page_id", using: :btree
  end

  create_table "page_links", force: :cascade do |t|
    t.string   "text"
    t.integer  "src_page_id", null: false
    t.integer  "dst_page_id", null: false
    t.datetime "created_at",  null: false
    t.datetime "updated_at",  null: false
    t.index ["dst_page_id"], name: "index_page_links_on_dst_page_id", using: :btree
    t.index ["src_page_id"], name: "index_page_links_on_src_page_id", using: :btree
  end

  create_table "pages", force: :cascade do |t|
    t.string   "text",       null: false
    t.string   "url",        null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.integer  "book_id",    null: false
    t.string   "monsters"
    t.index ["book_id"], name: "index_pages_on_book_id", using: :btree
    t.index ["url"], name: "index_pages_on_url", unique: true, using: :btree
  end

  add_foreign_key "aventures", "books"
  add_foreign_key "aventures", "pages"
  add_foreign_key "books", "pages", column: "first_page_id"
  add_foreign_key "page_links", "pages", column: "dst_page_id"
  add_foreign_key "page_links", "pages", column: "src_page_id"
  add_foreign_key "pages", "books"
end
