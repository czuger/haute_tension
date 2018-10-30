require 'test_helper'

class BugsControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user, items: {} )
    @user.update!( current_adventure_id: @adventure.id )
  end

  test 'should get new bug' do
    get new_bug_url
    assert_response :success
  end

  test 'should create bug' do
    post bugs_url, params: { bug: { info: :foo, page_id: @book.first_page_id } }
    assert_redirected_to new_bug_url
  end

end
