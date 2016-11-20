require 'test_helper'

class NotesControllerTest < ActionDispatch::IntegrationTest

  def setup
    @adventure = create( :adventure )
  end

  test "should get edit" do
    get edit_note_url( @adventure )
    assert_response :success
  end

  test "should get update" do
    patch note_url( @adventure )
    assert_redirected_to adventure_play_url( @adventure )
  end

end
