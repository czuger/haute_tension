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

ActiveRecord::Schema.define(version: 20170112156653) do

  # These are extensions that must be enabled in order to support this database
  enable_extension "plpgsql"

  create_table "adventures", force: :cascade do |t|
    t.integer  "book_id",                           null: false
    t.integer  "page_id",                           null: false
    t.integer  "hp",                                null: false
    t.integer  "strength",                          null: false
    t.integer  "gold",                              null: false
    t.integer  "waterskins",         default: 2,    null: false
    t.integer  "waterskins_max",     default: 2
    t.integer  "rations",            default: 4
    t.boolean  "charisma_avaliable", default: true
    t.datetime "created_at",                        null: false
    t.datetime "updated_at",                        null: false
    t.string   "notes"
    t.integer  "hp_max",                            null: false
    t.integer  "strength_max",                      null: false
    t.index ["book_id"], name: "index_adventures_on_book_id", using: :btree
    t.index ["page_id"], name: "index_adventures_on_page_id", using: :btree
  end

  create_table "adventures_items", id: false, force: :cascade do |t|
    t.integer "adventure_id", null: false
    t.integer "item_id",      null: false
    t.index ["adventure_id", "item_id"], name: "index_adventures_items_on_adventure_id_and_item_id", unique: true, using: :btree
  end

  create_table "books", force: :cascade do |t|
    t.string   "name",          null: false
    t.integer  "first_page_id"
    t.datetime "created_at",    null: false
    t.datetime "updated_at",    null: false
    t.index ["first_page_id"], name: "index_books_on_first_page_id", using: :btree
  end

  create_table "downloaded_books", force: :cascade do |t|
    t.string   "name",       null: false
    t.string   "url",        null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["name"], name: "index_downloaded_books_on_name", unique: true, using: :btree
  end

  create_table "downloaded_pages", force: :cascade do |t|
    t.string   "url",                null: false
    t.integer  "downloaded_book_id", null: false
    t.datetime "created_at",         null: false
    t.datetime "updated_at",         null: false
    t.index ["downloaded_book_id"], name: "index_downloaded_pages_on_downloaded_book_id", using: :btree
    t.index ["url"], name: "index_downloaded_pages_on_url", unique: true, using: :btree
  end

  create_table "fight_monsters", force: :cascade do |t|
    t.integer "adventure_id", null: false
    t.integer "monster_id",   null: false
    t.integer "hp",           null: false
    t.index ["adventure_id"], name: "index_fight_monsters_on_adventure_id", using: :btree
    t.index ["monster_id"], name: "index_fight_monsters_on_monster_id", using: :btree
  end

  create_table "game_logs", force: :cascade do |t|
    t.integer  "adventure_id", null: false
    t.integer  "page_id",      null: false
    t.integer  "log_type",     null: false
    t.string   "log_data"
    t.datetime "created_at",   null: false
    t.datetime "updated_at",   null: false
    t.index ["adventure_id"], name: "index_game_logs_on_adventure_id", using: :btree
    t.index ["page_id"], name: "index_game_logs_on_page_id", using: :btree
  end

  create_table "items", force: :cascade do |t|
    t.string   "name",       null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
  end

  create_table "monsters", force: :cascade do |t|
    t.string   "name",       null: false
    t.integer  "strength",   null: false
    t.integer  "hp",         null: false
    t.integer  "adjustment"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
  end

  create_table "monsters_pages", id: false, force: :cascade do |t|
    t.integer "page_id",    null: false
    t.integer "monster_id", null: false
    t.index ["page_id", "monster_id"], name: "index_monsters_pages_on_page_id_and_monster_id", unique: true, using: :btree
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
    t.index ["book_id"], name: "index_pages_on_book_id", using: :btree
    t.index ["url"], name: "index_pages_on_url", unique: true, using: :btree
  end

  add_foreign_key "adventures", "books"
  add_foreign_key "adventures", "pages"
  add_foreign_key "books", "pages", column: "first_page_id"
  add_foreign_key "downloaded_pages", "downloaded_books"
  add_foreign_key "fight_monsters", "adventures"
  add_foreign_key "fight_monsters", "monsters"
  add_foreign_key "game_logs", "adventures"
  add_foreign_key "game_logs", "pages"
  add_foreign_key "page_links", "pages", column: "dst_page_id"
  add_foreign_key "page_links", "pages", column: "src_page_id"
  add_foreign_key "pages", "books"
end
